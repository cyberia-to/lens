// ---
// tags: genies, source
// crystal-type: source
// crystal-domain: comp
// ---

//! F_q field arithmetic for the CSIDH-512 prime.
//!
//! q = 4 * l_1 * l_2 * ... * l_74 - 1 where l_i are the first 74 odd primes.
//! q is 511 bits, stored as 8 x u64 limbs in little-endian order.
//! All arithmetic is schoolbook with Barrett reduction.

/// CSIDH-512 prime: q = 4 * 3 * 5 * 7 * ... * 587 - 1
/// 511-bit prime stored as 8 little-endian u64 limbs.
pub const PRIME: [u64; 8] = [
    0x1b81b90533c6c87b,
    0xc2721bf457aca835,
    0x516730cc1f0b4f25,
    0xa7aac6c567f35507,
    0x5afbfcc69322c9cd,
    0xb42d083aedc88c42,
    0xfc8ab0d15e3e4c4a,
    0x65b48e8f740f89bf,
];

/// (q - 1) / 2 for Euler criterion (Legendre symbol).
const PRIME_MINUS_1_HALF: [u64; 8] = [
    0x8dc0dc8299e3643d,
    0xe1390dfa2bd6541a,
    0xa8b398660f85a792,
    0xd3d56362b3f9aa83,
    0x2d7dfe63499164e6,
    0x5a16841d76e44621,
    0xfe455868af1f2625,
    0x32da4747ba07c4df,
];

/// Barrett reduction constant: mu = floor(2^1024 / q).
/// mu has 514 bits, stored as 9 u64 limbs.
const BARRETT_MU: [u64; 9] = [
    0x87471983e2ffb9d4,
    0xabb862a1eabde765,
    0x48b72f84899eca3b,
    0xdb7e0542b77624de,
    0xafaeb264ca1bb35a,
    0xba24269dff081925,
    0x5d6cec71e0fac030,
    0x845f1c9d401fac7f,
    0x0000000000000002,
];

/// A field element in F_q, represented as 8 little-endian u64 limbs.
/// The value is always in [0, q).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fq {
    pub limbs: [u64; 8],
}

impl Fq {
    /// The zero element.
    pub const ZERO: Fq = Fq { limbs: [0; 8] };

    /// The multiplicative identity.
    pub const ONE: Fq = Fq {
        limbs: [1, 0, 0, 0, 0, 0, 0, 0],
    };

    /// Construct from raw limbs. Caller must ensure value < q.
    pub const fn from_limbs(limbs: [u64; 8]) -> Self {
        Fq { limbs }
    }

    /// Construct from a small u64 value.
    pub const fn from_u64(v: u64) -> Self {
        Fq {
            limbs: [v, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    /// Check if this element is zero.
    pub fn is_zero(&self) -> bool {
        let mut acc = 0u64;
        let mut i = 0;
        while i < 8 {
            acc |= self.limbs[i];
            i += 1;
        }
        acc == 0
    }

    /// Modular addition: (a + b) mod q.
    pub fn add(a: &Fq, b: &Fq) -> Fq {
        let mut result = [0u64; 8];
        let mut carry = 0u64;
        let mut i = 0;
        while i < 8 {
            let (s1, c1) = a.limbs[i].overflowing_add(b.limbs[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            result[i] = s2;
            carry = (c1 as u64) + (c2 as u64);
            i += 1;
        }
        // Subtract q if result >= q
        let mut borrow = 0u64;
        let mut sub = [0u64; 8];
        i = 0;
        while i < 8 {
            let (d1, b1) = result[i].overflowing_sub(PRIME[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            sub[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
            i += 1;
        }
        if carry >= borrow {
            Fq { limbs: sub }
        } else {
            Fq { limbs: result }
        }
    }

    /// Modular subtraction: (a - b) mod q.
    pub fn sub(a: &Fq, b: &Fq) -> Fq {
        let mut result = [0u64; 8];
        let mut borrow = 0u64;
        let mut i = 0;
        while i < 8 {
            let (d1, b1) = a.limbs[i].overflowing_sub(b.limbs[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            result[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
            i += 1;
        }
        if borrow != 0 {
            let mut carry = 0u64;
            i = 0;
            while i < 8 {
                let (s1, c1) = result[i].overflowing_add(PRIME[i]);
                let (s2, c2) = s1.overflowing_add(carry);
                result[i] = s2;
                carry = (c1 as u64) + (c2 as u64);
                i += 1;
            }
        }
        Fq { limbs: result }
    }

    /// Modular negation: q - a (or 0 if a == 0).
    pub fn neg(a: &Fq) -> Fq {
        if a.is_zero() {
            return Fq::ZERO;
        }
        Fq::sub(&Fq::ZERO, a)
    }

    /// 64x64 -> 128-bit unsigned multiply.
    #[inline(always)]
    fn umul128(a: u64, b: u64) -> (u64, u64) {
        let full = (a as u128) * (b as u128);
        (full as u64, (full >> 64) as u64)
    }

    /// Schoolbook 8x8 multiplication producing 16 limbs.
    fn mul_wide(a: &[u64; 8], b: &[u64; 8]) -> [u64; 16] {
        let mut result = [0u64; 16];
        let mut i = 0;
        while i < 8 {
            let mut carry = 0u64;
            let mut j = 0;
            while j < 8 {
                let (lo, hi) = Self::umul128(a[i], b[j]);
                let (s1, c1) = result[i + j].overflowing_add(lo);
                let (s2, c2) = s1.overflowing_add(carry);
                result[i + j] = s2;
                carry = hi + (c1 as u64) + (c2 as u64);
                j += 1;
            }
            result[i + 8] = carry;
            i += 1;
        }
        result
    }

    /// Multiply N-limb number `a` (length `a_len`) by M-limb number `b` (length `b_len`),
    /// writing to `out` (which must have length >= a_len + b_len).
    /// Only computes limbs from index `out_start` onwards (skip low limbs for Barrett).
    fn mul_n_m(a: &[u64], b: &[u64], out: &mut [u64]) {
        let a_len = a.len();
        let b_len = b.len();
        // Zero the output
        let mut k = 0;
        while k < out.len() {
            out[k] = 0;
            k += 1;
        }
        let mut i = 0;
        while i < a_len {
            if a[i] == 0 {
                i += 1;
                continue;
            }
            let mut carry = 0u64;
            let mut j = 0;
            while j < b_len {
                let idx = i + j;
                if idx >= out.len() {
                    break;
                }
                let (lo, hi) = Self::umul128(a[i], b[j]);
                let (s1, c1) = out[idx].overflowing_add(lo);
                let (s2, c2) = s1.overflowing_add(carry);
                out[idx] = s2;
                carry = hi + (c1 as u64) + (c2 as u64);
                j += 1;
            }
            let final_idx = i + b_len;
            if final_idx < out.len() {
                out[final_idx] = out[final_idx].wrapping_add(carry);
            }
            i += 1;
        }
    }

    /// Barrett reduction of a 16-limb product to 8 limbs mod q.
    ///
    /// Algorithm (Barrett, k=8 limbs, b=2^64):
    ///   1. q1 = floor(x / b^(k-1)) = x >> 448        (up to 9 limbs)
    ///   2. q2 = q1 * mu                               (up to 18 limbs)
    ///   3. q3 = floor(q2 / b^(k+1)) = q2 >> 576       (up to 9 limbs)
    ///   4. r1 = x mod b^(k+1) = x & (2^576 - 1)      (9 limbs)
    ///   5. r2 = (q3 * q) mod b^(k+1)                  (9 limbs)
    ///   6. r = r1 - r2 (add b^(k+1) if negative)
    ///   7. while r >= q: r -= q                        (at most 2 times)
    fn barrett_reduce(t: &[u64; 16]) -> Fq {
        // Step 1: q1 = t >> 448 (t[7..15], up to 9 limbs)
        let q1: [u64; 9] = [
            t[7], t[8], t[9], t[10], t[11], t[12], t[13], t[14], t[15],
        ];

        // Step 2: q2 = q1 * mu (9 x 9 = up to 18 limbs)
        let mut q2 = [0u64; 18];
        Self::mul_n_m(&q1, &BARRETT_MU, &mut q2);

        // Step 3: q3 = q2 >> 576 = q2[9..17] (up to 9 limbs)
        let q3: [u64; 9] = [
            q2[9], q2[10], q2[11], q2[12], q2[13], q2[14], q2[15], q2[16], q2[17],
        ];

        // Step 4: r1 = t mod 2^576 (low 9 limbs of t)
        let r1: [u64; 9] = [
            t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8],
        ];

        // Step 5: r2 = (q3 * q) mod 2^576
        // We only need the low 9 limbs of q3 * PRIME
        let mut r2_full = [0u64; 17]; // q3 (9 limbs) * PRIME (8 limbs) = 17 limbs
        Self::mul_n_m(&q3, &PRIME, &mut r2_full);
        // Take low 9 limbs
        let r2: [u64; 9] = [
            r2_full[0], r2_full[1], r2_full[2], r2_full[3],
            r2_full[4], r2_full[5], r2_full[6], r2_full[7], r2_full[8],
        ];

        // Step 6: r = r1 - r2 (mod 2^576)
        let mut r = [0u64; 9];
        let mut borrow = 0u64;
        let mut i = 0;
        while i < 9 {
            let (d1, b1) = r1[i].overflowing_sub(r2[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            r[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
            i += 1;
        }
        // If borrow != 0, r wrapped around. Add 2^576 conceptually.
        // Since we work mod 2^576, the wrapping is already handled by u64 arithmetic.
        // The result r is correct mod 2^576.

        // Step 7: while r >= q, subtract q (at most 2-3 times)
        // We compare 9-limb r with 8-limb q (q[8] = 0 implicitly)
        let mut count = 0;
        loop {
            // Check if r >= q (9-limb comparison)
            let mut ge = true;
            let mut k: usize = 9;
            while k > 0 {
                k -= 1;
                let qk = if k < 8 { PRIME[k] } else { 0 };
                if r[k] > qk {
                    break; // r > q at this limb, so r >= q
                }
                if r[k] < qk {
                    ge = false;
                    break; // r < q at this limb
                }
                // equal, continue to next lower limb
            }
            if !ge {
                break;
            }

            // Subtract q
            let mut bw = 0u64;
            i = 0;
            while i < 9 {
                let qk = if i < 8 { PRIME[i] } else { 0 };
                let (d1, b1) = r[i].overflowing_sub(qk);
                let (d2, b2) = d1.overflowing_sub(bw);
                r[i] = d2;
                bw = (b1 as u64) + (b2 as u64);
                i += 1;
            }

            count += 1;
            if count > 5 {
                // Safety: Barrett should need at most 2-3 corrections
                break;
            }
        }

        Fq {
            limbs: [r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]],
        }
    }

    /// Modular multiplication: (a * b) mod q.
    pub fn mul(a: &Fq, b: &Fq) -> Fq {
        let wide = Self::mul_wide(&a.limbs, &b.limbs);
        Self::barrett_reduce(&wide)
    }

    /// Modular squaring: a^2 mod q.
    pub fn square(a: &Fq) -> Fq {
        Fq::mul(a, a)
    }

    /// Modular exponentiation by a 512-bit exponent given as 8 limbs.
    fn pow_limbs(base: &Fq, exp: &[u64; 8]) -> Fq {
        let mut result = Fq::ONE;
        let mut i: usize = 8;
        while i > 0 {
            i -= 1;
            let mut bit = 63;
            loop {
                result = Fq::square(&result);
                if (exp[i] >> bit) & 1 == 1 {
                    result = Fq::mul(&result, base);
                }
                if bit == 0 {
                    break;
                }
                bit -= 1;
            }
        }
        result
    }

    /// Modular inverse via Fermat's little theorem: a^(q-2) mod q.
    pub fn inv(a: &Fq) -> Fq {
        let qm2: [u64; 8] = [
            0x1b81b90533c6c879,
            0xc2721bf457aca835,
            0x516730cc1f0b4f25,
            0xa7aac6c567f35507,
            0x5afbfcc69322c9cd,
            0xb42d083aedc88c42,
            0xfc8ab0d15e3e4c4a,
            0x65b48e8f740f89bf,
        ];
        Self::pow_limbs(a, &qm2)
    }

    /// Legendre symbol: returns 1 if a is a quadratic residue (QR),
    /// -1 (as i8) if a is a non-residue, 0 if a is zero.
    pub fn legendre(a: &Fq) -> i8 {
        if a.is_zero() {
            return 0;
        }
        let r = Self::pow_limbs(a, &PRIME_MINUS_1_HALF);
        if r == Fq::ONE {
            1
        } else {
            -1
        }
    }

    /// Square root via a^((q+1)/4) for q ≡ 3 (mod 4).
    pub fn sqrt(a: &Fq) -> Option<Fq> {
        if a.is_zero() {
            return Some(Fq::ZERO);
        }
        let qp1_over4 = Self::shr2_512(&[
            0x1b81b90533c6c87c,
            0xc2721bf457aca835,
            0x516730cc1f0b4f25,
            0xa7aac6c567f35507,
            0x5afbfcc69322c9cd,
            0xb42d083aedc88c42,
            0xfc8ab0d15e3e4c4a,
            0x65b48e8f740f89bf,
        ]);
        let r = Self::pow_limbs(a, &qp1_over4);
        if Fq::square(&r) == *a {
            Some(r)
        } else {
            None
        }
    }

    fn shr2_512(v: &[u64; 8]) -> [u64; 8] {
        let mut result = [0u64; 8];
        let mut i = 0;
        while i < 7 {
            result[i] = (v[i] >> 2) | (v[i + 1] << 62);
            i += 1;
        }
        result[7] = v[7] >> 2;
        result
    }

    fn geq(a: &[u64; 8], b: &[u64; 8]) -> bool {
        let mut i: usize = 8;
        while i > 0 {
            i -= 1;
            if a[i] > b[i] {
                return true;
            }
            if a[i] < b[i] {
                return false;
            }
        }
        true
    }

    /// Reduce a value mod q if it is >= q.
    pub fn reduce(limbs: &[u64; 8]) -> Fq {
        if Self::geq(limbs, &PRIME) {
            let mut result = [0u64; 8];
            let mut borrow = 0u64;
            let mut i = 0;
            while i < 8 {
                let (d1, b1) = limbs[i].overflowing_sub(PRIME[i]);
                let (d2, b2) = d1.overflowing_sub(borrow);
                result[i] = d2;
                borrow = (b1 as u64) + (b2 as u64);
                i += 1;
            }
            Fq { limbs: result }
        } else {
            Fq { limbs: *limbs }
        }
    }
}
