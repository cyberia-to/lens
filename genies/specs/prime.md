# CSIDH prime

## form

q = 4 · ℓ₁ · ℓ₂ · ... · ℓₙ - 1

where ℓ₁ < ℓ₂ < ... < ℓₙ are distinct small odd primes.

## requirements

| property | reason |
|----------|--------|
| q ≡ 3 (mod 4) | supersingular curve E: y² = x³ + x exists over F_q |
| q prime | F_q is a field |
| many small ℓᵢ in q+1 | efficient ℓ-isogeny computation (Vélu: O(ℓ)) |
| |cl(O)| ≈ √q | class group large enough for security |
| q ≈ 2^512 or 2^1024 | match target security level |

## why not Goldilocks

Goldilocks: p = 2⁶⁴ - 2³² + 1. factorization of p+1 = 2⁶⁴ - 2³² + 2:

p + 1 = 2 · (2⁶³ - 2³¹ + 1)

the factor 2⁶³ - 2³¹ + 1 is a large prime. there are no small odd prime factors in p+1. this means no efficient ℓ-isogenies exist over F_p.

CSIDH requires q+1 = 4 · ℓ₁ · ℓ₂ · ... · ℓₙ — many small factors. Goldilocks was chosen for NTT (2-adic structure in p-1), which is the opposite of what CSIDH needs (smooth structure in q+1).

this is a fundamental algebraic incompatibility, not a design limitation.

## CSIDH-512 parameters

the standard CSIDH-512 prime:

q = 4 · 3 · 5 · 7 · 11 · 13 · ... · 587 - 1

using the first 74 odd primes: ℓ₁ = 3, ℓ₂ = 5, ..., ℓ₇₄ = 587.

q ≈ 2^511 (512-bit prime).

security: NIST level 1 (≈ 128-bit classical, ≈ 64-bit quantum).

## F_q arithmetic

since q ≈ 2^512, field arithmetic requires multi-limb representation:

| operation | method | cost (64-bit limbs) |
|-----------|--------|---------------------|
| add | modular addition | 8 limbs + carry chain |
| sub | modular subtraction | 8 limbs + borrow chain |
| mul | schoolbook or Karatsuba | 8×8 limbs → 16 limbs + reduction |
| inv | Fermat (q^(q-2)) | ~512 squarings + ~256 multiplications |
| sqrt | Tonelli-Shanks | ~512 squarings |

this is ~8× slower than Goldilocks per field operation (64-bit native vs 512-bit multi-limb). acceptable because genies operations are infrequent (key exchange, not bulk computation).

## constant-time implementation

CSIDH key exchange MUST be constant-time to prevent timing side-channels. the dCTIDH (dummy-free constant-time CSIDH) variant achieves this by:

1. fixed number of isogeny steps per prime ℓᵢ
2. no dummy operations (unlike original CTIDH)
3. rejection sampling for uniform key distribution

constant-time is enforced at the F_q arithmetic level: no branches on secret data, no variable-time multiplication.
