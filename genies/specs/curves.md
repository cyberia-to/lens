# supersingular curves

## curve equation

the base curve for CSIDH:

```
E_0: y^2 = x^3 + x    over F_q
```

this curve is supersingular because q = 3 (mod 4) and the equation has the form y^2 = x^3 + x (a = 1, b = 0 in short Weierstrass).

all curves in the CSIDH isogeny graph have the Montgomery form:

```
E_A: By^2 = x^3 + Ax^2 + x    over F_q
```

with B = 1 for the standard CSIDH setting. the curve is determined by the single coefficient A in F_q.

## point representation

**affine coordinates**: (x, y) in F_q x F_q satisfying the curve equation, plus the point at infinity O.

**projective coordinates**: (X : Y : Z) where x = X/Z, y = Y/Z. the point at infinity is (0 : 1 : 0). projective coordinates avoid inversions in intermediate computations.

**XZ-only (Montgomery ladder)**: (X : Z) representing x = X/Z. sufficient for scalar multiplication and isogeny computation when the full y-coordinate is not needed.

## point addition

for Montgomery curves E_A: y^2 = x^3 + Ax^2 + x, using XZ coordinates:

**differential addition** (given P, Q, and P-Q):
```
U = (X_P - Z_P)(X_Q + Z_Q)
V = (X_P + Z_P)(X_Q - Z_Q)
X_{P+Q} = Z_{P-Q} * (U + V)^2
Z_{P+Q} = X_{P-Q} * (U - V)^2
```

**doubling**:
```
S = (X_P + Z_P)^2
D = (X_P - Z_P)^2
X_{2P} = S * D
Z_{2P} = (S - D) * (S + ((A+2)/4)(S - D))
```

cost: 5 F_q multiplications + 4 F_q additions per differential step.

## scalar multiplication

Montgomery ladder for [k]P using XZ coordinates:

```
input: scalar k, point P = (X_P : Z_P)
R_0 = (1 : 0)   // point at infinity
R_1 = (X_P : Z_P)
for each bit b_i of k (high to low):
    if b_i = 0: R_1 = R_0 + R_1, R_0 = 2*R_0
    if b_i = 1: R_0 = R_0 + R_1, R_1 = 2*R_1
output: R_0
```

constant-time: the ladder performs the same operations regardless of bit value (conditional swap, not branch).

## j-invariant

for Montgomery curve E_A:

```
j(E_A) = 256 * (A^2 - 3)^3 / (A^2 - 4)
```

two curves are isomorphic over F_q if and only if they have the same j-invariant. the j-invariant is the canonical identifier for a curve up to isomorphism.

## supersingularity

E is supersingular over F_q iff #E(F_q) = q + 1. for CSIDH curves, this holds by construction: the Frobenius trace is 0.

the set of supersingular j-invariants over F_q forms the vertices of the isogeny graph. genies navigates this graph via the class group action.
