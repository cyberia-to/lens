//! Lane-mode grinding at 20–26 bits: every backend against the CPU.
use cyber_lens_rspcs::backend::{Backend, Cpu, Inject};
fn main() {
    let w = cyber_lens_gpu::webgpu().expect("webgpu");
    let m = cyber_lens_gpu::metal().expect("metal");
    let mut r = 0x1234_5678_u64;
    for trial in 0..12 {
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        let bits = 20 + (trial % 6) as u32;
        let lane = (r % 9) as usize;
        let mut t: [u64; 16] =
            core::array::from_fn(|i| (r.wrapping_mul(i as u64 + 3)) % 0xFFFF_FFFF_0000_0001);
        t[lane] = 0;
        let c = Cpu.grind(&t, Inject::lane(lane), bits);
        let g = w.backend.grind(&t, Inject::lane(lane), bits);
        let mm = m.backend.grind(&t, Inject::lane(lane), bits);
        println!(
            "bits {bits} lane {lane}: cpu {c} webgpu {g} metal {mm} {}",
            if c == g && c == mm { "ok" } else { "MISMATCH" }
        );
    }
}
