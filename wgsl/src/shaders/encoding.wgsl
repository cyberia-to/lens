// ── nebu/encoding ────────────────────────────────────────────────
//
// Byte ↔ Goldilocks element conversion for GPU compute.
// 7 bytes per element, little-endian.
// Requires: field.wgsl
//
// WGSL cannot pass storage pointers to functions. Encoding helpers
// work on pre-fetched u32 words. The entry-point shader reads from
// the binding and calls these.

// Extract one byte from a u32 word.
fn extract_byte(word: u32, lane: u32) -> u32 {
    return (word >> (lane * 8u)) & 0xFFu;
}

// Decode: return canonical (lo, hi) of a field element — ready to
// write as two u32s to output storage.
fn gl_decode_8(val_lo: u32, val_hi: u32) -> vec2<u32> {
    return gl_canon(val_lo, val_hi);
}
