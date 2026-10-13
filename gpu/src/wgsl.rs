//! WGSL source: Goldilocks over `vec2<u32>` limbs (`x` low, `y` high; WGSL
//! has no 64-bit integers), hemera's permutation, and the kernels. Every
//! kernel binds at most two storage buffers (PowerVR drivers hang above
//! four). Every stored value is canonical (`< p`), matching the CPU.
//!
//! Bindings: 0 = constants (storage, read: 144 round constants then the
//! 16-lane diagonal), 1 = states (storage, read-write) or the grinding
//! result (storage, atomic), 2 = parameters (uniform).

pub const SOURCE: &str = r#"
const EPS: u32 = 0xFFFFFFFFu;

@group(0) @binding(0) var<storage, read> K: array<vec2<u32>, 160>;

fn ge64(a: vec2<u32>, b: vec2<u32>) -> bool {
    return a.y > b.y || (a.y == b.y && a.x >= b.x);
}

// wrapping 64-bit add; returns (sum, carry)
fn add64(a: vec2<u32>, b: vec2<u32>) -> vec3<u32> {
    let lo = a.x + b.x;
    let c0 = select(0u, 1u, lo < a.x);
    let h1 = a.y + b.y;
    let c1 = select(0u, 1u, h1 < a.y);
    let h2 = h1 + c0;
    let c2 = select(0u, 1u, h2 < h1);
    return vec3<u32>(lo, h2, c1 | c2);
}

// wrapping 64-bit subtract
fn sub64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x - b.x;
    let br = select(0u, 1u, a.x < b.x);
    return vec2<u32>(lo, a.y - b.y - br);
}

const PV: vec2<u32> = vec2<u32>(1u, 0xFFFFFFFFu);
const EPSV: vec2<u32> = vec2<u32>(0xFFFFFFFFu, 0u);

fn canon(a: vec2<u32>) -> vec2<u32> {
    if ge64(a, PV) { return sub64(a, PV); }
    return a;
}

// canonical in, canonical out
fn gadd(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let s = add64(a, b);
    var r = s.xy;
    if s.z == 1u { r = add64(r, EPSV).xy; }
    return canon(r);
}

fn mul32(a: u32, b: u32) -> vec2<u32> {
    let a0 = a & 0xFFFFu;
    let a1 = a >> 16u;
    let b0 = b & 0xFFFFu;
    let b1 = b >> 16u;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = p01 + (p00 >> 16u);
    let mid2 = (mid & 0xFFFFu) + p10;
    let lo = (mid2 << 16u) | (p00 & 0xFFFFu);
    let hi = p11 + (mid >> 16u) + (mid2 >> 16u);
    return vec2<u32>(lo, hi);
}

// any limbs in, canonical out
fn gmul(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let ll = mul32(a.x, b.x);
    let lh = mul32(a.x, b.y);
    let hl = mul32(a.y, b.x);
    let hh = mul32(a.y, b.y);
    // 128-bit product r3:r2:r1:r0
    let s1a = ll.y + lh.x;
    let c1a = select(0u, 1u, s1a < ll.y);
    let r1 = s1a + hl.x;
    let c1b = select(0u, 1u, r1 < s1a);
    let s2a = lh.y + hl.y;
    let c2a = select(0u, 1u, s2a < lh.y);
    let s2b = s2a + hh.x;
    let c2b = select(0u, 1u, s2b < s2a);
    let r2 = s2b + c1a + c1b;
    let c2c = select(0u, 1u, r2 < s2b);
    let r3 = hh.y + c2a + c2b + c2c;
    // x = lo + 2^64 (r2 + 2^32 r3) ≡ lo − r3 + r2·(2^32 − 1)
    let lo = vec2<u32>(ll.x, r1);
    var t0 = sub64(lo, vec2<u32>(r3, 0u));
    if lo.y == 0u && lo.x < r3 { t0 = sub64(t0, EPSV); }
    var t1 = vec2<u32>(0u, 0u);
    if r2 != 0u { t1 = vec2<u32>(0u - r2, r2 - 1u); }
    let s = add64(t0, t1);
    var r = s.xy;
    if s.z == 1u { r = add64(r, EPSV).xy; }
    return canon(r);
}

fn gsqn(x: vec2<u32>, n: u32) -> vec2<u32> {
    var y = x;
    for (var i = 0u; i < n; i++) { y = gmul(y, y); }
    return y;
}

// x^(p-2), 72 multiplications; 0 maps to 0
fn ginv(x: vec2<u32>) -> vec2<u32> {
    let t2 = gmul(gmul(x, x), x);
    let t3 = gmul(gmul(t2, t2), x);
    let t6 = gmul(gsqn(t3, 3u), t3);
    let t12 = gmul(gsqn(t6, 6u), t6);
    let t24 = gmul(gsqn(t12, 12u), t12);
    let t30 = gmul(gsqn(t24, 6u), t6);
    let t31 = gmul(gmul(t30, t30), x);
    let u = gmul(t31, t31);
    let c = gmul(u, x);
    return gmul(gsqn(u, 32u), c);
}

fn pow7(x: vec2<u32>) -> vec2<u32> {
    let x2 = gmul(x, x);
    let x3 = gmul(x2, x);
    let x4 = gmul(x2, x2);
    return gmul(x3, x4);
}

fn mds_ext(s: ptr<function, array<vec2<u32>, 16>>) {
    for (var c = 0u; c < 4u; c++) {
        let o = 4u * c;
        let x0 = (*s)[o]; let x1 = (*s)[o + 1u]; let x2 = (*s)[o + 2u]; let x3 = (*s)[o + 3u];
        let t01 = gadd(x0, x1);
        let t23 = gadd(x2, x3);
        let t0123 = gadd(t01, t23);
        let t01123 = gadd(t0123, x1);
        let t01233 = gadd(t0123, x3);
        (*s)[o + 3u] = gadd(t01233, gadd(x0, x0));
        (*s)[o + 1u] = gadd(t01123, gadd(x2, x2));
        (*s)[o] = gadd(t01123, t01);
        (*s)[o + 2u] = gadd(t01233, t23);
    }
    for (var k = 0u; k < 4u; k++) {
        let sum = gadd(gadd((*s)[k], (*s)[4u + k]), gadd((*s)[8u + k], (*s)[12u + k]));
        (*s)[k] = gadd((*s)[k], sum);
        (*s)[4u + k] = gadd((*s)[4u + k], sum);
        (*s)[8u + k] = gadd((*s)[8u + k], sum);
        (*s)[12u + k] = gadd((*s)[12u + k], sum);
    }
}

fn full_round(s: ptr<function, array<vec2<u32>, 16>>, r: u32) {
    for (var i = 0u; i < 16u; i++) { (*s)[i] = pow7(gadd((*s)[i], K[r * 16u + i])); }
    mds_ext(s);
}

fn permute(s: ptr<function, array<vec2<u32>, 16>>) {
    mds_ext(s);
    for (var r = 0u; r < 4u; r++) { full_round(s, r); }
    for (var r = 0u; r < 16u; r++) {
        (*s)[0] = ginv(gadd((*s)[0], K[128u + r]));
        var sum = (*s)[0];
        for (var i = 1u; i < 16u; i++) { sum = gadd(sum, (*s)[i]); }
        for (var i = 0u; i < 16u; i++) { (*s)[i] = gadd(gmul((*s)[i], K[144u + i]), sum); }
    }
    for (var r = 4u; r < 8u; r++) { full_round(s, r); }
}

fn shl64(v: vec2<u32>, n: u32) -> vec2<u32> {
    if n == 0u { return v; }
    if n >= 64u { return vec2<u32>(0u, 0u); }
    if n >= 32u { return vec2<u32>(0u, v.x << (n - 32u)); }
    return vec2<u32>(v.x << n, (v.y << n) | (v.x >> (32u - n)));
}

fn shr64(v: vec2<u32>, n: u32) -> vec2<u32> {
    if n == 0u { return v; }
    if n >= 64u { return vec2<u32>(0u, 0u); }
    if n >= 32u { return vec2<u32>(v.y >> (n - 32u), 0u); }
    return vec2<u32>((v.x >> n) | (v.y << (32u - n)), v.y >> n);
}

struct PermParams { n: u32, row: u32, a: u32, b: u32 };
struct GrindParams {
    tpl: array<vec4<u32>, 8>,
    base: vec2<u32>,
    mask: vec2<u32>,
    lo_mask: vec2<u32>,
    lo: u32,
    lo_shift: u32,
    hi: u32,
    hi_shift: u32,
    count: u32,
    row: u32,
};

@group(0) @binding(1) var<storage, read_write> ST: array<vec2<u32>>;
@group(0) @binding(2) var<uniform> PP: PermParams;

@compute @workgroup_size(64)
fn hemera_permute(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * PP.row;
    if gid >= PP.n { return; }
    var s: array<vec2<u32>, 16>;
    for (var i = 0u; i < 16u; i++) { s[i] = ST[16u * gid + i]; }
    permute(&s);
    for (var i = 0u; i < 16u; i++) { ST[16u * gid + i] = s[i]; }
}
"#;

/// The grinding module (same field and permutation, its own bindings).
pub fn grind_source() -> String {
    let field = &SOURCE[..SOURCE.find("struct PermParams").expect("marker")];
    let structs = &SOURCE[SOURCE.find("struct PermParams").expect("marker")
        ..SOURCE.find("@group(0) @binding(1)").expect("marker")];
    format!(
        "{field}{structs}{}",
        r#"
@group(0) @binding(1) var<storage, read_write> FOUND: atomic<u32>;
@group(0) @binding(2) var<uniform> G: GrindParams;

@compute @workgroup_size(64)
fn hemera_grind(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * G.row;
    if gid >= G.count { return; }
    let n = add64(G.base, vec2<u32>(gid, 0u)).xy;
    var s: array<vec2<u32>, 16>;
    for (var i = 0u; i < 8u; i++) {
        s[2u * i] = G.tpl[i].xy;
        s[2u * i + 1u] = G.tpl[i].zw;
    }
    let lo = vec2<u32>(n.x & G.lo_mask.x, n.y & G.lo_mask.y);
    s[G.lo] = gadd(s[G.lo], shl64(lo, G.lo_shift));
    if G.hi_shift < 64u { s[G.hi] = gadd(s[G.hi], shr64(n, G.hi_shift)); }
    permute(&s);
    if (s[0].x & G.mask.x) == 0u && (s[0].y & G.mask.y) == 0u {
        atomicMin(&FOUND, gid);
    }
}
"#
    )
}

/// The sponge module: per item, overwrite the rate lanes block by block
/// and permute (bindings: 0 constants, 1 lanes, 2 parameters, 3 digests —
/// three storage buffers).
pub fn sponge_source() -> String {
    let field = &SOURCE[..SOURCE.find("struct PermParams").expect("marker")];
    format!(
        "{field}{}",
        r#"
struct SpongeParams {
    init: array<vec4<u32>, 8>,
    rate: u32,
    blocks: u32,
    items: u32,
    row: u32,
};

@group(0) @binding(1) var<storage, read> LANES: array<vec2<u32>>;
@group(0) @binding(2) var<uniform> SP: SpongeParams;
@group(0) @binding(3) var<storage, read_write> OUT: array<vec2<u32>>;

@compute @workgroup_size(64)
fn hemera_sponge(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * SP.row;
    if gid >= SP.items { return; }
    var s: array<vec2<u32>, 16>;
    for (var i = 0u; i < 8u; i++) {
        s[2u * i] = SP.init[i].xy;
        s[2u * i + 1u] = SP.init[i].zw;
    }
    let base = gid * SP.rate * SP.blocks;
    for (var b = 0u; b < SP.blocks; b++) {
        for (var i = 0u; i < SP.rate; i++) { s[i] = LANES[base + b * SP.rate + i]; }
        permute(&s);
    }
    for (var i = 0u; i < 4u; i++) { OUT[4u * gid + i] = s[i]; }
}
"#
    )
}

/// The program module: a straight-line Fp3 program summed along lines
/// (bindings: 1 program — ops then constants, 2 columns then weights,
/// 3 per-thread results, 4 parameters; three storage buffers).
pub fn lines_source() -> String {
    let field = &SOURCE[..SOURCE.find("struct PermParams").expect("marker")];
    format!(
        "{field}{}",
        r#"
fn gsub(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let d = sub64(a, b);
    if ge64(a, b) { return d; }
    return add64(d, PV).xy;
}

struct F3 { a: vec2<u32>, b: vec2<u32>, c: vec2<u32> };

fn f3add(x: F3, y: F3) -> F3 { return F3(gadd(x.a, y.a), gadd(x.b, y.b), gadd(x.c, y.c)); }
fn f3sub(x: F3, y: F3) -> F3 { return F3(gsub(x.a, y.a), gsub(x.b, y.b), gsub(x.c, y.c)); }
fn f3neg(x: F3) -> F3 { let z = vec2<u32>(0u, 0u); return F3(gsub(z, x.a), gsub(z, x.b), gsub(z, x.c)); }
fn f3mulb(x: F3, k: vec2<u32>) -> F3 { return F3(gmul(x.a, k), gmul(x.b, k), gmul(x.c, k)); }
fn f3mul(x: F3, y: F3) -> F3 {
    let d0 = gmul(x.a, y.a);
    let d1 = gadd(gmul(x.a, y.b), gmul(x.b, y.a));
    let d2 = gadd(gadd(gmul(x.a, y.c), gmul(x.b, y.b)), gmul(x.c, y.a));
    let d3 = gadd(gmul(x.b, y.c), gmul(x.c, y.b));
    let d4 = gmul(x.c, y.c);
    return F3(gadd(d0, d3), gadd(gadd(d1, d3), d4), gadd(d2, d4));
}

struct LineParams {
    nops: u32,
    ncols: u32,
    len: u32,
    nx: u32,
    threads: u32,
    first: u32,
    output: u32,
    consts_at: u32,   // u32 offset of the constants in PROG
    weights_at: u32,  // vec2 offset of the weights in DATA
    row: u32,
    pad0: u32,
    pad1: u32,
    xs: array<vec4<u32>, 8>,
};

@group(0) @binding(1) var<storage, read> PROG: array<u32>;
@group(0) @binding(2) var<storage, read> DATA: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read_write> RES: array<vec2<u32>>;
@group(0) @binding(4) var<uniform> LP: LineParams;

fn konst(k: u32) -> F3 {
    let o = LP.consts_at + 6u * k;
    return F3(vec2<u32>(PROG[o], PROG[o + 1u]), vec2<u32>(PROG[o + 2u], PROG[o + 3u]), vec2<u32>(PROG[o + 4u], PROG[o + 5u]));
}

fn xval(k: u32) -> vec2<u32> {
    let v = LP.xs[k / 2u];
    if k % 2u == 0u { return v.xy; }
    return v.zw;
}

@compute @workgroup_size(64)
fn prog_lines(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * LP.row;
    if gid >= LP.threads { return; }
    let i = LP.first + gid / LP.nx;
    let x = xval(gid % LP.nx);
    var r: array<F3, 160>;
    for (var n = 0u; n < LP.nops; n++) {
        let code = PROG[4u * n];
        var opnd = array<u32, 2>(PROG[4u * n + 1u], PROG[4u * n + 2u]);
        var v: array<F3, 2>;
        for (var q = 0u; q < 2u; q++) {
            let o = opnd[q];
            let t = o >> 30u;
            let k = o & 0x3FFFFFFFu;
            if t == 2u {
                v[q] = r[k];
            } else if t == 1u {
                v[q] = konst(k);
            } else {
                let at = 3u * (k * LP.len + 2u * i);
                let a = F3(DATA[at], DATA[at + 1u], DATA[at + 2u]);
                let b = F3(DATA[at + 3u], DATA[at + 4u], DATA[at + 5u]);
                v[q] = f3add(a, f3mulb(f3sub(b, a), x));
            }
            if code == 3u || code == 4u { break; }
        }
        var res: F3;
        if code == 0u { res = f3add(v[0], v[1]); }
        else if code == 1u { res = f3sub(v[0], v[1]); }
        else if code == 2u { res = f3mul(v[0], v[1]); }
        else if code == 3u { res = f3neg(v[0]); }
        else { res = f3mulb(v[0], konst(opnd[1] & 0x3FFFFFFFu).a); }
        r[PROG[4u * n + 3u]] = res;
    }
    // the output operand
    let o = LP.output;
    var out: F3;
    if (o >> 30u) == 2u { out = r[o & 0x3FFFFFFFu]; } else if (o >> 30u) == 1u { out = konst(o & 0x3FFFFFFFu); } else {
        let at = 3u * ((o & 0x3FFFFFFFu) * LP.len + 2u * i);
        let a = F3(DATA[at], DATA[at + 1u], DATA[at + 2u]);
        let b = F3(DATA[at + 3u], DATA[at + 4u], DATA[at + 5u]);
        out = f3add(a, f3mulb(f3sub(b, a), x));
    }
    let wa = LP.weights_at + 3u * i;
    let g = F3(DATA[wa], DATA[wa + 1u], DATA[wa + 2u]);
    let res = f3mul(g, out);
    RES[3u * gid] = res.a;
    RES[3u * gid + 1u] = res.b;
    RES[3u * gid + 2u] = res.c;
}
"#
    )
}
