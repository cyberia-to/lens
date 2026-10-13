//! WGSL kernels for the device-resident paths (line sessions, coset NTTs,
//! streamed leaves, Merkle levels). Every kernel binds at most four
//! storage buffers (PowerVR drivers hang above four); Goldilocks over
//! `vec2<u32>` limbs as in [`crate::wgsl`].

/// The field, hemera's permutation and Fp3 (no bindings of their own
/// except the hemera constants `K`, binding 0, used by permuting kernels).
fn common() -> String {
    let field = &crate::wgsl::SOURCE[..crate::wgsl::SOURCE
        .find("struct PermParams")
        .expect("marker")];
    let lines = crate::wgsl::lines_source();
    let f3 = &lines
        [lines.find("fn gsub(").expect("gsub")..lines.find("struct LineParams").expect("marker")];
    format!("{field}{f3}")
}

fn module(body: &str) -> String {
    format!("{}{body}", common())
}

pub fn lines_session() -> String {
    module(
        r#"
struct SP {
    nops: u32, ncols: u32, stride: u32, nx: u32,
    threads: u32, first: u32, output: u32, consts_at: u32,
    row: u32, p0: u32, p1: u32, p2: u32,
    xs: array<vec4<u32>, 8>,
};
@group(0) @binding(1) var<storage, read> PROG: array<u32>;
@group(0) @binding(2) var<storage, read> COLS: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read> WTS: array<vec2<u32>>;
@group(0) @binding(4) var<storage, read_write> RES: array<vec2<u32>>;
@group(0) @binding(5) var<uniform> P: SP;

fn konst(k: u32) -> F3 {
    let o = P.consts_at + 6u * k;
    return F3(vec2<u32>(PROG[o], PROG[o + 1u]), vec2<u32>(PROG[o + 2u], PROG[o + 3u]), vec2<u32>(PROG[o + 4u], PROG[o + 5u]));
}
fn xval(k: u32) -> vec2<u32> {
    let v = P.xs[k / 2u];
    if k % 2u == 0u { return v.xy; }
    return v.zw;
}
fn rd(o: u32, i: u32, x: vec2<u32>, r: ptr<function, array<F3, 160>>) -> F3 {
    let t = o >> 30u;
    let k = o & 0x3FFFFFFFu;
    if t == 2u { return (*r)[k]; }
    if t == 1u { return konst(k); }
    let at = 3u * (k * P.stride + 2u * i);
    let a = F3(COLS[at], COLS[at + 1u], COLS[at + 2u]);
    let b = F3(COLS[at + 3u], COLS[at + 4u], COLS[at + 5u]);
    return f3add(a, f3mulb(f3sub(b, a), x));
}

@compute @workgroup_size(64)
fn prog_lines_s(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * P.row;
    if gid >= P.threads { return; }
    let i = P.first + gid / P.nx;
    let x = xval(gid % P.nx);
    var r: array<F3, 160>;
    for (var n = 0u; n < P.nops; n++) {
        let code = PROG[4u * n];
        let a = rd(PROG[4u * n + 1u], i, x, &r);
        var v: F3;
        if code == 0u { v = f3add(a, rd(PROG[4u * n + 2u], i, x, &r)); }
        else if code == 1u { v = f3sub(a, rd(PROG[4u * n + 2u], i, x, &r)); }
        else if code == 2u { v = f3mul(a, rd(PROG[4u * n + 2u], i, x, &r)); }
        else if code == 3u { v = f3neg(a); }
        else { v = f3mulb(a, konst(PROG[4u * n + 2u] & 0x3FFFFFFFu).a); }
        r[PROG[4u * n + 3u]] = v;
    }
    let out = rd(P.output, i, x, &r);
    let g = F3(WTS[3u * i], WTS[3u * i + 1u], WTS[3u * i + 2u]);
    let res = f3mul(g, out);
    RES[3u * gid] = res.a;
    RES[3u * gid + 1u] = res.b;
    RES[3u * gid + 2u] = res.c;
}
"#,
    )
}

pub fn fold_cols() -> String {
    module(
        r#"
struct FP { alpha: array<vec4<u32>, 2>, ncols: u32, stride: u32, halfn: u32, row: u32 };
@group(0) @binding(1) var<storage, read> SRC: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> DST: array<vec2<u32>>;
@group(0) @binding(3) var<uniform> F: FP;

@compute @workgroup_size(64)
fn fold_cols(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * F.row;
    if gid >= F.ncols * F.halfn { return; }
    let c = gid / F.halfn;
    let i = gid % F.halfn;
    let s = 3u * (c * F.stride + 2u * i);
    let v = F3(SRC[s], SRC[s + 1u], SRC[s + 2u]);
    let w = F3(SRC[s + 3u], SRC[s + 4u], SRC[s + 5u]);
    let al = F3(F.alpha[0].xy, F.alpha[0].zw, F.alpha[1].xy);
    let r = f3add(v, f3mul(al, f3sub(w, v)));
    let d = 3u * (c * F.stride + i);
    DST[d] = r.a;
    DST[d + 1u] = r.b;
    DST[d + 2u] = r.c;
}
"#,
    )
}

/// Coset NTT kernels: scatter (coset powers, bit reversal), a block of
/// the first `BLOCK_LOG` stages in workgroup memory, single stages.
pub fn ntt() -> String {
    module(
        r#"
struct NP { log_n: u32, span: u32, cosets: u32, row: u32 };
@group(0) @binding(1) var<storage, read> COEF: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> DATA: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read> SHIFTS: array<vec2<u32>>;
@group(0) @binding(4) var<uniform> N: NP;
@group(0) @binding(5) var<storage, read> TW: array<vec2<u32>>;

fn gpow(b0: vec2<u32>, e0: u32) -> vec2<u32> {
    var r = vec2<u32>(1u, 0u);
    var b = b0;
    var e = e0;
    while e != 0u {
        if (e & 1u) == 1u { r = gmul(r, b); }
        b = gmul(b, b);
        e = e >> 1u;
    }
    return r;
}

@compute @workgroup_size(64)
fn ntt_scatter(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * N.row;
    let n = 1u << N.log_n;
    if gid >= n * N.cosets { return; }
    let s = gid >> N.log_n;
    let i = gid & (n - 1u);
    var r = 0u;
    if N.log_n > 0u { r = reverseBits(i) >> (32u - N.log_n); }
    DATA[s * n + r] = gmul(COEF[i], gpow(SHIFTS[s], i));
}

@compute @workgroup_size(64)
fn ntt_stage(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * N.row;
    let n = 1u << N.log_n;
    let per = n >> 1u;
    if gid >= per * N.cosets { return; }
    let s = gid / per;
    let b = gid - s * per;
    let i = b & (N.span - 1u);
    let j = (b - i) << 1u;
    let at = s * n + j + i;
    let t = gmul(TW[N.span + i], DATA[at + N.span]);
    let u = DATA[at];
    DATA[at + N.span] = gsub(u, t);
    DATA[at] = gadd(u, t);
}

var<workgroup> BLK: array<vec2<u32>, 512>;

// stages 0..N.span (N.span ≤ 9) of every block of 2^N.span elements
@compute @workgroup_size(256)
fn ntt_block(@builtin(local_invocation_index) tid: u32, @builtin(workgroup_id) wg: vec3<u32>) {
    let size = 1u << N.span;
    let halfb = size >> 1u;
    let blk = wg.x + wg.y * N.row;
    // the 2-D grid may hold more workgroups than blocks (uniform per
    // workgroup: the early return keeps the barriers uniform)
    if blk >= ((N.cosets << N.log_n) >> N.span) { return; }
    let base = blk * size;
    if tid < halfb {
        BLK[tid] = DATA[base + tid];
        BLK[tid + halfb] = DATA[base + tid + halfb];
    }
    for (var st = 0u; st < N.span; st++) {
        workgroupBarrier();
        if tid < halfb {
            let h = 1u << st;
            let i = tid & (h - 1u);
            let j = ((tid - i) << 1u) + i;
            let t = gmul(TW[h + i], BLK[j + h]);
            let u = BLK[j];
            BLK[j + h] = gsub(u, t);
            BLK[j] = gadd(u, t);
        }
    }
    workgroupBarrier();
    if tid < halfb {
        DATA[base + tid] = BLK[tid];
        DATA[base + tid + halfb] = BLK[tid + halfb];
    }
}
"#,
    )
}

pub fn coset_leaves() -> String {
    module(
        r#"
struct LP {
    init: array<vec4<u32>, 8>,
    k: u32, per: u32, g: u32, w: u32,
    members: u32, limbs: u32, spb: u32, blocks: u32,
    rate: u32, row: u32, p0: u32, p1: u32,
};
@group(0) @binding(1) var<storage, read> VALS: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> OUT: array<vec2<u32>>;
@group(0) @binding(3) var<uniform> L: LP;

@compute @workgroup_size(64)
fn coset_leaves(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x + id.y * L.row;
    if local >= L.p1 { return; }
    let gid = L.p0 + local;
    if gid >= L.g * L.per { return; }
    let c = gid / L.per;
    let a = gid % L.per;
    var s: array<vec2<u32>, 16>;
    for (var i = 0u; i < 8u; i++) {
        s[2u * i] = L.init[i].xy;
        s[2u * i + 1u] = L.init[i].zw;
    }
    let total = L.members * L.w;
    for (var blk = 0u; blk < L.blocks; blk++) {
        var lane = 0u;
        for (var j = 0u; j < L.spb; j++) {
            let t = blk * L.spb + j;
            if t >= total { break; }
            let m = t / L.w;
            let tt = t % L.w;
            let u = a + tt * L.per;
            for (var l = 0u; l < L.limbs; l++) {
                s[lane + l] = VALS[((m * L.limbs + l) * L.g + c) * L.k + u];
            }
            lane += L.limbs;
        }
        for (var i = lane; i < L.rate; i++) { s[i] = vec2<u32>(0u, 0u); }
        permute(&s);
    }
    for (var i = 0u; i < 4u; i++) { OUT[4u * gid + i] = s[i]; }
}
"#,
    )
}

pub fn merkle_level() -> String {
    module(
        r#"
struct MP { count: u32, arity: u32, row: u32, p0: u32, tag: vec2<u32>, p1: vec2<u32> };
@group(0) @binding(1) var<storage, read> SRC: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> DST: array<vec2<u32>>;
@group(0) @binding(3) var<uniform> M: MP;

@compute @workgroup_size(64)
fn merkle_level(@builtin(global_invocation_id) id: vec3<u32>) {
    let gid = id.x + id.y * M.row + M.p0;
    if gid >= M.count { return; }
    var s: array<vec2<u32>, 16>;
    for (var i = 0u; i < 16u; i++) { s[i] = vec2<u32>(0u, 0u); }
    for (var i = 0u; i < 4u * M.arity; i++) { s[i] = SRC[4u * M.arity * gid + i]; }
    if M.arity == 2u { s[8] = M.tag; }
    permute(&s);
    for (var i = 0u; i < 4u; i++) { DST[4u * gid + i] = s[i]; }
}
"#,
    )
}
