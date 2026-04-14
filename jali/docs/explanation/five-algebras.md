# five algebras: the complete arithmetic stack

the cyber network runs five distinct execution algebras. each is a separate implementation because each occupies a different regime of computation where the cost structure is fundamentally different. trying to force one algebra to do another's job wastes orders of magnitude.

## the five

| algebra | repo | domain | structure | characteristic object |
|---------|------|--------|-----------|-----------------------|
| F_p | nebu | prime field | field (all ops) | Goldilocks element (u64) |
| F_2 tower | kuro | binary field | field (all ops) | 128-bit tower element (u128) |
| (min, +) | trop | tropical semiring | semiring (no inverse) | weighted edge (u64) |
| F_q isogeny | genies | isogeny group action | group action | supersingular curve (512-bit) |
| R_q | jali | polynomial ring | ring (structured) | ring element (1024 x u64) |

five names, five repos, five fundamentally different algebraic structures.

## nebu — the scalar backbone

nebu provides Goldilocks field arithmetic: F_p where p = 2^64 - 2^32 + 1. every element fits in a u64. addition is one instruction. multiplication is one 64-bit multiply plus a fast reduction.

nebu is the foundation. every other algebra ultimately connects to Goldilocks:
- kuro's binary proofs verify over F_p
- trop's optimality certificates check over F_p
- genies' isogeny witnesses fold into F_p limbs
- jali's ring elements are vectors of F_p scalars

the proof system (zheng) accumulates constraints over F_p. Goldilocks is the verification language.

## kuro — the binary regime

kuro provides F_2 tower arithmetic: GF(2) through GF(2^128) via seven quadratic extensions. addition is XOR. multiplication is Karatsuba through the tower.

the justification: bitwise operations in F_p cost ~32 constraints (bit decomposition). in F_2, they cost 1 constraint. the 32x gap is algebraic — characteristic 2 makes bits native.

workloads: BitNet 1-bit AI inference (AND + popcount), boolean circuits, AES/SHA verification, quantized SpMV for consensus.

## trop — the optimization regime

trop provides tropical semiring arithmetic: addition is min, multiplication is standard addition. there is no additive inverse — this is a semiring, not a field.

the justification: encoding min(a, b) in F_p requires bit decomposition for comparison — ~10 constraints. native tropical: 1 operation. the 10x gap justifies a separate algebra for optimization workloads.

workloads: shortest paths (Dijkstra, Floyd-Warshall), assignment problems (Hungarian), sequence alignment (Viterbi), network flow. the dual (max, +) semiring covers longest paths and max-reliability routing.

## genies — the post-quantum regime

genies provides isogeny group action arithmetic over a 512-bit CSIDH prime. the class group of a supersingular elliptic curve acts on the set of curves — a commutative group action that enables non-interactive post-quantum protocols.

the justification: the CSIDH prime q = 4 * l_1 * l_2 * ... * l_74 - 1 requires smooth q+1, which is algebraically incompatible with NTT-friendly primes. Goldilocks cannot serve as the isogeny prime. genies is the one module with a foreign prime — because mathematics forbids the alternative.

workloads: post-quantum key exchange (CSIDH DH), verifiable random functions (VRF), verifiable delay functions (VDF), threshold signatures, stealth addresses.

## jali — the polynomial ring regime

jali provides polynomial ring arithmetic: R_q = F_p[x]/(x^n+1) over Goldilocks. elements are vectors of n field elements with negacyclic convolution as multiplication.

the justification: one ring multiply = 3n scalar multiplies (via NTT). at n = 1024, that is 3072 scalar operations. encoding ring multiplication as 3072 separate constraints wastes three orders of magnitude. ring-aware computation — NTT batching, automorphism permutations, noise tracking — recovers the gap.

workloads: fully homomorphic encryption (TFHE bootstrapping, BFV/BGV), post-quantum key encapsulation (ML-KEM/Kyber), ring-aware polynomial commitments.

## why five and not one

a single "universal" algebra could represent all five domains — embed binary operations in F_p, encode min as comparison circuits, simulate ring multiply as n^2 scalar operations, represent isogeny curves over a different prime. this works. it is also catastrophically slow.

the cost gaps:

```
domain          operation         native cost    F_p emulation cost    gap
────────        ─────────         ───────────    ──────────────────    ───
binary          XOR               1              32                    32x
tropical        min(a, b)         1              ~10                   10x
polynomial ring ring multiply     3n             n^2                   ~340x (via NTT)
isogeny         group action      O(sqrt(q))     impossible over F_p   ∞
```

the isogeny case is the extreme: there is no way to instantiate CSIDH over Goldilocks. the ring case is the most practical: 340x for the most common FHE operation. the binary and tropical cases are the most pervasive: every bitwise or comparison operation pays the tax.

five algebras is not complexity for complexity's sake. it is the minimum number required to avoid order-of-magnitude performance penalties across the workloads that matter.

## how they compose

the five algebras are not islands. they compose through the proof system:

```
execution layer:
  nebu   — scalar arithmetic (polynomials, hashes, accumulators)
  kuro   — binary arithmetic (BitNet, boolean circuits)
  trop   — optimization (routing, assignment, scheduling)
  genies — isogeny computation (key exchange, VRF)
  jali   — ring arithmetic (FHE, lattice KEM)

         ↓ witnesses ↓

verification layer (zheng over F_p):
  nebu constraints  — direct
  kuro constraints  — binary-to-prime boundary (~766 constraints/crossing)
  trop constraints  — LP duality certificate (verified over F_p)
  genies constraints — F_q element folded into 8 Goldilocks limbs
  jali constraints  — ring-aware PCS (NTT batching, noise accumulator)
```

every algebra produces witnesses. zheng verifies all witnesses over F_p (Goldilocks). the verification is uniform even though the execution is heterogeneous.

## the design principle

each algebra exists because it satisfies three criteria:

1. **distinct cost structure**: at least 10x gap vs emulation in F_p
2. **significant workload**: the domain covers real computation that the network needs
3. **clean interface**: the algebra produces witnesses that zheng can verify

if a new computational regime emerges with a 10x+ gap and a real workload, it would justify a sixth algebra. until then, five is complete.

## see also

- [[polynomial-rings]] — the algebraic structure of jali's domain
- [[negacyclic-ntt]] — the algorithm that makes ring multiplication practical
- [[fhe-overview]] — the primary application driving jali
- [[lattice-security]] — the security foundation for ring-based cryptography
