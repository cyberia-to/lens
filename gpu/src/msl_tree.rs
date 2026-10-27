//! MSL kernels of the hemera byte trees (lens `merkle::Tree`): appended to
//! the library source after the field and permutation in `msl`.

/// Hemera byte trees (lens `merkle::Tree`) over leaves cut from a codeword.
pub(crate) const BYTE_TREE: &str = r#"
// ── hemera byte trees: hash_leaf over codeword cosets, hash_node levels ──
struct ByteLeafParams {
    uint leaves;   // leaf count (threads)
    uint width;    // symbols per leaf: symbol j + t·leaves, t < width
    uint limbs;    // u64 limbs per symbol
    uint bytes;    // leaf length = width · limbs · 8
    uint flags;    // re-derivation flags (chunk, root when one leaf)
    uint pad;
};

// word i of leaf j's byte string, canonical
static inline ulong leaf_word(device const ulong* sym, constant ByteLeafParams& p, uint j, uint i) {
    uint t = i / p.limbs;
    uint l = i - t * p.limbs;
    ulong v = sym[((ulong)j + (ulong)t * p.leaves) * p.limbs + l];
    return v >= P ? v - P : v;
}

// the 7 bytes at offset o (o + 7 <= bytes), little-endian
static inline ulong lane7(device const ulong* sym, constant ByteLeafParams& p, uint j, uint o) {
    uint w = o >> 3;
    uint sh = (o & 7) * 8;
    ulong v = leaf_word(sym, p, j, w) >> sh;
    if (sh > 8) v |= leaf_word(sym, p, j, w + 1) << (64 - sh);
    return v & 0x00FFFFFFFFFFFFFFUL;
}

kernel void byte_leaves(device const ulong* sym [[buffer(0)]],
                        device ulong* out [[buffer(1)]],
                        constant ByteLeafParams& p [[buffer(2)]],
                        uint gid [[thread_position_in_grid]]) {
    if (gid >= p.leaves) return;
    ulong s[1][16];
    for (int i = 0; i < 16; i++) s[0][i] = 0;
    uint full = p.bytes / 56;
    for (uint b = 0; b < full; b++) {
        for (uint q = 0; q < 8; q++) s[0][q] = gadd(s[0][q], lane7(sym, p, gid, b * 56 + 7 * q));
        permute_k<1>(s);
    }
    for (uint q = 0; q < 8; q++) {
        ulong v = 0;
        for (uint b = 0; b < 7; b++) {
            uint pos = full * 56 + 7 * q + b;
            ulong byte = pos < p.bytes
                ? ((leaf_word(sym, p, gid, pos >> 3) >> ((pos & 7) * 8)) & 0xffUL)
                : (pos == p.bytes ? 1UL : 0UL);
            v |= byte << (8 * b);
        }
        s[0][q] = gadd(s[0][q], v);
    }
    s[0][10] = p.bytes;
    permute_k<1>(s);
    ulong d0 = s[0][0], d1 = s[0][1], d2 = s[0][2], d3 = s[0][3];
    for (int i = 0; i < 16; i++) s[0][i] = 0;
    s[0][0] = d0; s[0][1] = d1; s[0][2] = d2; s[0][3] = d3;
    s[0][8] = gid;
    s[0][9] = p.flags;
    permute_k<1>(s);
    for (int i = 0; i < 4; i++) out[4 * (ulong)gid + i] = s[0][i];
}

struct NodeParams {
    uint count;   // nodes of this level
    uint flags;   // parent, and root at the top
};

kernel void tree_nodes(device const ulong* src [[buffer(0)]],
                       device ulong* dst [[buffer(1)]],
                       constant NodeParams& p [[buffer(2)]],
                       uint gid [[thread_position_in_grid]]) {
    if (gid >= p.count) return;
    ulong s[1][16];
    for (int i = 0; i < 16; i++) s[0][i] = 0;
    device const ulong* c = src + 8 * (ulong)gid;
    for (int i = 0; i < 8; i++) s[0][i] = c[i];
    s[0][9] = p.flags;
    permute_k<1>(s);
    for (int i = 0; i < 4; i++) dst[4 * (ulong)gid + i] = s[0][i];
}
"#;
