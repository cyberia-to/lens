# isogenies: maps between curves

an isogeny is a morphism between elliptic curves that preserves the group structure. it sends the identity to the identity and respects the addition law: phi(P + Q) = phi(P) + phi(Q). genies computes isogenies between supersingular curves to realize the class group action.

## the kernel determines the map

every isogeny phi: E -> E' is determined (up to isomorphism of E') by its kernel — the set of points that map to the identity. for an l-isogeny, the kernel has exactly l elements.

```
phi: E -> E' = E / ker(phi)
```

given a point P of order l on E, the cyclic subgroup <P> = {O, P, [2]P, ..., [l-1]P} is a valid kernel. the quotient curve E/<P> is the codomain E'. Velu's formulas make this computation explicit.

## Velu's formulas

given a Montgomery curve E_A: y^2 = x^3 + Ax^2 + x and a point P of order l generating the kernel, the codomain coefficient A' is:

```
kernel points: {P, [2]P, ..., [(l-1)/2]P}    (half the non-trivial kernel)

pi_0  = product(x([i]P))   for i = 1 to (l-1)/2
sigma_0 = sum(x([i]P))     for i = 1 to (l-1)/2

A' = (pi_0 * A - 3 * sigma_0) / pi_0
```

the key insight: we only need the x-coordinates of the kernel points, and only half of them (the other half have the same x-coordinates due to the [-1] symmetry). this makes XZ-only arithmetic sufficient.

## cost of a single isogeny

computing an l-isogeny requires:
1. **kernel enumeration**: compute [2]P, [3]P, ..., [(l-1)/2]P. this takes (l-3)/2 point additions/doublings, each costing ~5 F_q multiplications.
2. **symmetric functions**: accumulate pi_0 (product of x-coordinates) and sigma_0 (sum of x-coordinates). this takes (l-1)/2 F_q multiplications.
3. **codomain**: evaluate the Velu formula. a few F_q multiplications plus an inversion.

total: O(l) F_q multiplications. for CSIDH-512, the primes range from l = 3 (trivial) to l = 587 (modest).

## image of a point

when we need to push a point Q through the isogeny (to set up the next step), the image has x-coordinate:

```
x(phi(Q)) = x(Q) * product((x(Q)*x([i]P) - 1)^2 / (x(Q) - x([i]P))^2)
```

for i = 1 to (l-1)/2. this evaluation costs O(l) F_q multiplications per point pushed through.

## the kernel polynomial

the kernel polynomial is:

```
h(x) = product(x - x([i]P))    for i = 1 to (l-1)/2
```

degree (l-1)/2. this polynomial encodes the isogeny completely — given h(x) and the domain curve, the codomain and the map can be recovered. the kernel polynomial appears in the verification circuit: zheng checks that it has the right degree and that its roots satisfy the curve equation.

## isogeny composition

the class group action is a composition of isogenies:

```
E_0 --phi_1--> E_1 --phi_2--> E_2 --...--> E_k
```

each phi_i is an l_i-isogeny for a different prime l_i. the composition phi_k o ... o phi_1 is the full action. genies computes this iteratively: after each isogeny, the curve changes, and the next isogeny is computed on the new curve.

this is why the action takes time proportional to the sum of the exponents: each nonzero exponent e_i requires |e_i| isogenies of degree l_i, applied sequentially.

## finding kernel points

to compute an l_i-isogeny from E_A, genies needs a point of order l_i:

1. sample random x in F_q
2. compute s = x^3 + Ax^2 + x
3. check if s is a square in F_q (Euler criterion: s^((q-1)/2) = 1 or -1)
4. if a square: P = (x, sqrt(s)) is on E_A. if not: P is on the twist.
5. cofactor multiply: Q = [(q+1)/l_i] * P
6. if Q = O, retry. otherwise Q has order l_i.

the sign of the exponent e_i determines whether we use a point on E_A (positive) or on the twist (negative). this is how the class group action distinguishes the two directions in the isogeny graph.

## sqrt-Velu

for large primes l, Velu's formulas cost O(l). Bernstein-De Feo-Leroux-Smith's sqrt-Velu algorithm reduces this to O(sqrt(l) log(l)) by factoring the kernel polynomial using a baby-step giant-step approach. for CSIDH-512 where the largest prime is 587, classical Velu is sufficient. sqrt-Velu becomes important at larger parameter sets (CSIDH-1024, CSIDH-1792).

## see also

- [[supersingular-curves]] — the curves that isogenies connect
- [[class-group]] — how isogenies compose into a group action
- [[verification]] — proving isogeny correctness via zheng
