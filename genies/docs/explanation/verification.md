# verification: folding F_q into F_p

genies computes over a 512-bit CSIDH prime. zheng proves over a 64-bit Goldilocks prime. the verification problem is: how do you prove that 512-bit arithmetic was performed correctly using a proof system that only understands 64-bit arithmetic? the answer is folding — decomposing every F_q element into 8 Goldilocks limbs and verifying the multi-limb arithmetic as F_p constraints.

## the representation

an F_q element x (512 bits) is decomposed into 8 limbs of 64 bits each:

```
x = x_0 + x_1 * 2^64 + x_2 * 2^128 + ... + x_7 * 2^448
```

each limb x_i is at most 2^64 - 1, which fits in a Goldilocks field element (p = 2^64 - 2^32 + 1 > 2^64 - 2^32, so any 64-bit value fits).

this decomposition is unique for values in [0, q). the prover provides the limbs as witness values; the verifier checks that they reassemble to a valid F_q element.

## verifying F_q addition

given a + b = c (mod q), the prover supplies limbs of a, b, and c. the verifier checks:

```
for i = 0 to 7:
    a_i + b_i + carry_in_i = c_i + carry_out_i * 2^64
```

where carry_out_i becomes carry_in_{i+1}. each carry is 0 or 1 (a single bit).

then the verifier checks that the result was reduced modulo q: if the unreduced sum exceeds q, a single subtraction of q was performed. this is one comparison (8 limb-wise comparisons) plus a conditional subtraction.

cost: approximately 8 F_p additions + 8 carry constraints + 8 comparison constraints. negligible.

## verifying F_q multiplication

given a * b = c (mod q), verification is more complex. the unreduced product a * b is a 1024-bit value requiring 16 limbs. the prover supplies:

1. the 16 limbs of the unreduced product z = a * b
2. the quotient q_hat such that z = q_hat * q + c
3. the 8 limbs of c (the reduced result)

the verifier checks:

```
step 1: z = schoolbook(a, b)    // 8x8 -> 16 limb multiplication
step 2: q_hat * q + c = z       // Barrett verification
step 3: 0 <= c < q              // range check
```

step 1 requires checking 64 limb-wise multiplication constraints (8 * 8 cross-products with carry propagation). step 2 requires another 8x8 multiplication (q_hat * q) plus an addition check. step 3 is a comparison.

cost: approximately 128 F_p multiplication constraints + carry propagation. this is the dominant cost in isogeny verification.

## verifying a single isogeny

an l-isogeny phi: E_A -> E_{A'} with kernel <P> is verified by checking:

1. **P is on E_A**: verify x(P)^3 + A * x(P)^2 + x(P) is a square in F_q. this is ~5 F_q multiplications = ~320 F_p constraints.

2. **P has order l**: verify [l]P = O via scalar multiplication check. the scalar multiplication is ~11 * log2(l) F_q muls, verified as ~700 * log2(l) F_p constraints.

3. **Velu formula is correct**: verify A' = (pi_0 * A - 3 * sigma_0) / pi_0 where pi_0 and sigma_0 are the symmetric functions of kernel x-coordinates. this requires checking each kernel point computation and the final formula.

4. **kernel polynomial**: verify the (l-1)/2 kernel points are computed correctly as consecutive multiples of P.

total per isogeny: O(l * 64) F_p constraints. for l = 3: ~200 constraints. for l = 587: ~40,000 constraints.

## verifying the full action

the class group action [a] * E_0 = E' is a chain of isogenies:

```
E_0 -> E_1 -> E_2 -> ... -> E_k = E'
```

the prover supplies the entire chain as a witness: each intermediate curve E_i, each kernel point P_i, and each isogeny step. the verifier checks each step independently.

total constraints for CSIDH-512 with average exponents:
- 74 primes, up to 5 isogenies each
- average prime ~ 150, average log2(l) ~ 7.2
- approximately 700,000 F_p constraints per action

this is expensive but within the capability of zheng. a single action proof takes seconds to generate and milliseconds to verify.

## the folding functions

genies provides three folding functions that the proof system calls:

```
fold_element(x: Fq) -> [Fp; 8]        // decompose F_q element into limbs
fold_mul(a: Fq, b: Fq) -> MulWitness   // produce multiplication witness
fold_action(e: &[i8], A: Fq) -> ActionWitness  // produce full action witness
```

the witness types contain all intermediate values needed for verification: limb decompositions, carry bits, quotients, intermediate curve coefficients, and kernel points.

## why this works despite the foreign prime

the CSIDH prime q is unrelated to Goldilocks p. there is no algebraic relationship between the two — no embedding, no homomorphism. the folding works purely through integer arithmetic: F_q elements are just large integers, and integer arithmetic can be verified over any field large enough to hold the limbs.

the cost is the overhead: every F_q operation becomes ~64 F_p operations (due to the 8x8 limb multiplication). but this is a constant factor, not a fundamental barrier. the proof is sound because integer arithmetic is the same regardless of which field the constraints are checked in.

## the full verification pipeline

```
genies: compute action(secret, E_0) = E'
  |
  | produces ActionWitness
  v
fold: decompose into Goldilocks constraints
  |
  | ActionWitness -> Vec<FpConstraint>
  v
zheng: verify constraints over F_p
  |
  | SNARK proof (~128 bytes)
  v
verifier: check proof (milliseconds)
```

the verifier never sees F_q values. it only sees Goldilocks field elements and checks that they satisfy polynomial constraints. the soundness of the proof system guarantees that valid proofs correspond to correct isogeny computations.

## see also

- [[csidh-prime]] — the foreign prime and why it exists
- [[class-group]] — the action being verified
- [[isogenies]] — the individual steps in the action chain
