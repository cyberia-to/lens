# isogeny computation

## definition

an l-isogeny phi: E -> E' is a non-constant morphism of elliptic curves with kernel of size l. given a point P of order l on E, the isogeny phi with kernel <P> maps E to a unique curve E' = E/<P>.

## Velu's formulas

given E_A: y^2 = x^3 + Ax^2 + x and a point P of order l on E_A, Velu's formulas compute the codomain curve E_{A'} and the map phi.

for the Montgomery form, the codomain coefficient A' is:

```
A' = (pi_0 * A - 3 * sigma_0) / pi_0
```

where pi_0 and sigma_0 are symmetric functions of the kernel points:

```
kernel points: {P, [2]P, ..., [(l-1)/2]P}   (half the non-trivial kernel)

pi_0 = product(x([i]P))  for i = 1 to (l-1)/2
sigma_0 = sum(x([i]P))   for i = 1 to (l-1)/2
```

## image of a point

for a point Q not in the kernel, the image phi(Q) has x-coordinate:

```
x(phi(Q)) = x(Q) * product((x(Q) * x([i]P) - 1)^2 / (x(Q) - x([i]P))^2)
```

for i = 1 to (l-1)/2.

## kernel polynomial

the kernel polynomial of an l-isogeny with kernel <P> is:

```
h(x) = product(x - x([i]P))  for i = 1 to (l-1)/2
```

degree: (l-1)/2. this polynomial determines the isogeny uniquely (up to isomorphism of the codomain).

## cost

per l-isogeny:
- computing kernel points: (l-1)/2 point doublings/additions ~ O(l) F_q muls
- evaluating Velu sums: (l-1)/2 multiplications ~ O(l) F_q muls
- total: O(l) F_q multiplications

for large l, sqrt-Velu (Bernstein-De Feo-Leroux-Smith) reduces this to O(sqrt(l) * log(l)).

## isogeny composition

the class group action composes multiple l-isogenies:

```
E_0 --l_1--> E_1 --l_2--> E_2 --...--> E_k
```

each step uses a different prime l_i. the composition phi_k o ... o phi_1 is the full action isogeny. genies computes this iteratively, updating the curve after each step.

## finding kernel points

to compute an l_i-isogeny from E_A:

1. sample random x in F_q
2. compute the point P = (x, y) on E_A (if x^3 + Ax^2 + x is a square)
3. compute Q = [(q+1)/l_i] * P (cofactor multiplication)
4. if Q = O, retry with new x
5. Q has order l_i and generates the kernel

this succeeds with probability ~ 1/2 per attempt (half of F_q elements give points on E_A vs its twist).
