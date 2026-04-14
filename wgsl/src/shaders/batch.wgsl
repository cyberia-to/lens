// ---
// tags: genies, wgsl
// crystal-type: source
// crystal-domain: comp
// ---

// Batch F_q operations — the GPU payoff.
// Each workgroup thread processes one independent F_q operation.

// Buffers: input pairs and output results.
// Each Fq is 16 x u32. A pair is 32 x u32. Output is 16 x u32.

@group(0) @binding(0) var<storage, read> input_a: array<u32>;
@group(0) @binding(1) var<storage, read> input_b: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@group(0) @binding(3) var<uniform> params: vec4<u32>;  // params.x = count

fn load_fq(buf: ptr<storage, array<u32>, read>, idx: u32) -> Fq {
    var f: Fq;
    let base = idx * 16u;
    for (var i = 0u; i < 16u; i++) {
        f.limbs[i] = (*buf)[base + i];
    }
    return f;
}

fn store_fq(idx: u32, f: Fq) {
    let base = idx * 16u;
    for (var i = 0u; i < 16u; i++) {
        output[base + i] = f.limbs[i];
    }
}

// Batch F_q multiplication: N independent muls.
// 64 threads per workgroup, each handles one mul.
@compute @workgroup_size(64)
fn batch_fq_mul(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.x { return; }

    let a = load_fq(&input_a, idx);
    let b = load_fq(&input_b, idx);
    let r = fq_mul(a, b);
    store_fq(idx, r);
}

// Batch F_q addition: N independent adds.
@compute @workgroup_size(64)
fn batch_fq_add(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.x { return; }

    let a = load_fq(&input_a, idx);
    let b = load_fq(&input_b, idx);
    let r = fq_add(a, b);
    store_fq(idx, r);
}
