# binary field arithmetic

an encyclopedia of the mathematics behind kuro's F₂ tower — from first principles to applications. every concept is grounded in the tower we implement: F₂ → F₂² → F₂⁴ → F₂⁸ → F₂¹⁶ → F₂³² → F₂⁶⁴ → F₂¹²⁸.

## foundations

- [[binary-fields]] — field axioms in characteristic 2, GF(2) as the algebra of bits, GF(2^n) as vector spaces
- [[tower-construction]] — quadratic extensions, why x² + x + α, the canonical generator, visual tower diagram

## algorithms

- [[karatsuba]] — Karatsuba multiplication through the tower, 3 multiplications instead of 4, recursive cost analysis
- [[inversion]] — Fermat's little theorem in GF(2^n), tower-recursive inversion, the Frobenius endomorphism

## data structures

- [[packed-operations]] — Packed128: 128 F₂ elements in one u128, SIMD-native addition/multiplication, the inner product kernel

## context

- [[applications]] — BitNet AI inference, binary STARKs, coding theory, AES, quantized SpMV
- [[f2-vs-fp]] — when to use binary fields (kuro) vs prime fields (nebu), cost model comparison, decision guide
