# tropical semiring

## definition

the tropical semiring T = (ℝ ∪ {+∞}, ⊕, ⊗) where:

- tropical addition: a ⊕ b = min(a, b)
- tropical multiplication: a ⊗ b = a + b (standard real addition)

| property | tropical add (⊕ = min) | tropical mul (⊗ = +) |
|----------|----------------------|---------------------|
| closure | min(a,b) ∈ T | a + b ∈ T |
| associativity | min(a, min(b,c)) = min(min(a,b), c) | (a+b)+c = a+(b+c) |
| commutativity | min(a,b) = min(b,a) | a+b = b+a |
| identity | +∞ (min(a, +∞) = a) | 0 (a + 0 = a) |
| distributivity | a + min(b,c) = min(a+b, a+c) | holds |

no additive inverse: there is no x such that min(a, x) = +∞ for all a ≠ +∞.
this is what makes it a semiring, not a ring or field.

## finite field encoding

over the [[Goldilocks field]] F_p (p = 2⁶⁴ - 2³² + 1):

- tropical elements: F_p values where p-1 represents +∞ (tropical zero)
- tropical add: min(a, b) via comparison. in [[nox]]: branch(lt(a, b), a, b)
- tropical mul: standard field addition (already exists as nox add pattern)

constraint cost for min(a, b) over F_p:
1. compute d = a - b (1 constraint)
2. decompose sign bit of d (range proof: ~8 constraints)
3. select result via branch (~1 constraint)
total: ~10 constraints per tropical addition

native tropical: 1 comparison operation. the 10× gap is the cost of encoding order into field arithmetic.

## dual semiring

the (max, +) semiring is the dual: a ⊕' b = max(a, b), a ⊗' b = a + b.

used for: longest path, maximum flow, max-plus spectral theory.

duality: replace every min with max. the structure is isomorphic via negation: min(-a, -b) = -max(a, b).

## idempotency

tropical addition is idempotent: a ⊕ a = min(a, a) = a.

this is the key structural difference from field addition (where a + a = 2a ≠ a unless a = 0). idempotency means tropical "summation" is a projection to the minimum, not an accumulation.

consequence: tropical linear algebra has fundamentally different behavior from classical linear algebra. tropical rank, tropical eigenvalues, tropical determinant all have distinct definitions.
