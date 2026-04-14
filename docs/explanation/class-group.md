# the class group action: why commutativity matters

the class group action is the algebraic heart of genies. it is the reason CSIDH can do non-interactive key exchange, verifiable random functions, and threshold protocols — all post-quantum, all without pairings. the key property is commutativity: the order in which you apply group elements does not matter.

## the action, precisely

let O = Z[pi] be the endomorphism ring of a supersingular elliptic curve over F_q, where pi is the Frobenius endomorphism. the ideal class group cl(O) is a finite abelian group that acts on the set Ell(O, pi) of supersingular curves sharing this endomorphism ring.

```
action: cl(O) x Ell(O, pi) -> Ell(O, pi)
[a] * E = E'
```

the action sends a curve E to another curve E'. it is:
- **free**: [a] * E = E implies [a] is the identity
- **transitive**: for any E, E' there exists [a] with [a] * E = E'
- **commutative**: [a] * ([b] * E) = [b] * ([a] * E) = [a + b] * E

transitivity means the class group connects every pair of supersingular curves. commutativity means the group action behaves like addition in a commutative group — the order of operations is irrelevant.

## why commutativity enables key exchange

Diffie-Hellman key exchange requires a commutative structure. in classical DH over a cyclic group G with generator g:

```
Alice: a, g^a     Bob: b, g^b
shared: g^(ab) = g^(ba)
```

in CSIDH over the class group acting on curves:

```
Alice: [a], [a]*E_0     Bob: [b], [b]*E_0
shared: [a]*([b]*E_0) = [b]*([a]*E_0) = [a+b]*E_0
```

the commutativity of the class group action directly replaces the commutativity of exponentiation. no pairing is needed. no interactive protocol is needed. Alice and Bob independently compute the same shared curve.

## why not just use isogeny graphs directly

SIDH (Supersingular Isogeny Diffie-Hellman) also uses isogenies but does NOT have a commutative group action. SIDH was broken in 2022 by Castryck-Decru precisely because the non-commutative structure leaks information about the secret isogeny through auxiliary torsion points.

CSIDH avoids this attack entirely because:
1. no auxiliary points are published
2. the commutative structure means the action is determined by the ideal class alone
3. the class group is abelian — there is no "twisting" attack

## ideal class representation

an element [a] of cl(O) is represented as an exponent vector:

```
[a] = [l_1]^e_1 * [l_2]^e_2 * ... * [l_n]^e_n
```

where [l_i] is the ideal class corresponding to the prime l_i dividing q+1, and e_i is an integer exponent. for CSIDH-512: n = 74 primes, exponents in {-5, ..., 5}, giving a key space of 11^74 ~ 2^256.

the exponent vector IS the secret key. it is 74 bytes (each exponent shifted to unsigned range). the public key is the resulting curve coefficient A — 64 bytes.

## the class number

the order of cl(O) — the class number h — satisfies:

```
h ~ sqrt(q) / 12    (Deuring's theorem, approximate)
```

for CSIDH-512: h ~ 2^256. this means there are approximately 2^256 distinct ideal classes, and therefore approximately 2^256 distinct supersingular curves reachable from E_0. the key space (11^74 ~ 2^256) is designed to cover the class group uniformly.

## security: the group action inverse problem

the GAIP (Group Action Inverse Problem): given E_0 and E' = [a] * E_0, find [a].

best known algorithms:
- **classical**: subexponential (index calculus on the class group), roughly L(1/2)
- **quantum**: Kuperberg's algorithm, subexponential O(2^(c * sqrt(log q)))
- **NOT broken by Shor**: Shor's algorithm applies to hidden subgroup problems in abelian groups, but the isogeny problem is a different structure

CSIDH-512 targets NIST level 1: approximately 128-bit classical security, approximately 64-bit quantum security. the subexponential quantum attack is the binding constraint on parameter selection.

## beyond key exchange

commutativity enables protocols that discrete log cannot:
- **VRF** (verifiable random function): prove the output is correct without revealing the key
- **VDF** (verifiable delay function): sequential computation that cannot be parallelized
- **threshold protocols**: distribute the secret key among n parties
- **stealth addresses**: derive unique addresses without interaction
- **blind signatures**: sign without seeing the message

these protocols live in mudra, which calls genies for the underlying group action.

## see also

- [[supersingular-curves]] — the set that the class group acts on
- [[isogenies]] — the maps that implement the action
- [[csidh-prime]] — why the prime must have smooth q+1
