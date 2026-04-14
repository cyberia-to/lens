# supersingular elliptic curves: the substrate

every computation in genies happens on supersingular elliptic curves. these are not arbitrary curves — they are a small, structured subset of all elliptic curves, connected to each other by isogenies. understanding what makes them special is the first step to understanding the entire construction.

## what is an elliptic curve

an elliptic curve over a field F_q is a set of points (x, y) satisfying a polynomial equation, together with a point at infinity O. the points form a group under a geometric addition law.

for genies, all curves are in Montgomery form:

```
E_A: y^2 = x^3 + Ax^2 + x    over F_q
```

the curve is determined entirely by the single coefficient A in F_q. this is important: public keys in CSIDH are just A values — 64 bytes.

## what makes a curve supersingular

an elliptic curve E over F_q is supersingular when #E(F_q) = q + 1. the number of rational points equals q + 1 exactly. this means the Frobenius trace t is zero (from Hasse's theorem: #E = q + 1 - t).

this is a strong constraint. most curves over F_q are ordinary — they have t != 0. supersingular curves are rare and special.

for the CSIDH prime q with q = 3 (mod 4), the base curve E_0: y^2 = x^3 + x is always supersingular. this is the starting point for all CSIDH computations.

## the endomorphism ring

every elliptic curve has an endomorphism ring — the ring of morphisms from the curve to itself. for ordinary curves, this ring is an order in an imaginary quadratic field. for supersingular curves, it is an order in a quaternion algebra.

the distinction matters because the endomorphism ring determines the class group that acts on the curve. supersingular curves over F_q (with their Frobenius endomorphism pi) have endomorphism ring O = Z[pi], and the class group cl(O) is the group that genies computes with.

## the isogeny graph

supersingular curves over F_q are connected by isogenies. for each small prime l dividing q+1, there are l+1 isogenies of degree l leaving each curve. the set of all supersingular curves and their l-isogenies forms a regular graph — the supersingular isogeny graph.

```
      E_1 ---l_1--- E_2
     / |              |
   l_2  l_1         l_2
   /    |              |
 E_0 --l_1--- E_3 ---l_1--- E_4
   \    |              |
   l_2  l_1         l_2
     \  |              |
      E_5 ---l_1--- E_6
```

this graph has remarkable properties:

- **Ramanujan**: the graph is an optimal expander. random walks mix rapidly, which is essential for the security of CSIDH (short walks look random to an adversary).
- **connected**: any supersingular curve can be reached from any other via a sequence of isogenies.
- **regular**: every vertex has the same number of edges for each degree.

the class group action navigates this graph. applying [a] to a curve E is walking from E along a specific path determined by the exponent vector.

## Montgomery form and the coefficient A

Montgomery form E_A: y^2 = x^3 + Ax^2 + x is preferred over short Weierstrass because:

1. **efficient arithmetic**: the Montgomery ladder computes scalar multiplication using only x-coordinates, saving half the field operations.
2. **compact representation**: a curve is a single field element A.
3. **twist compatibility**: the twist of E_A has the same Montgomery form structure, enabling the positive/negative exponent trick in the action.

the base curve has A = 0. after the class group action, the result curve has some A' in F_q. this A' is the public key in CSIDH key exchange, and the value that genies computes.

## point arithmetic in projective coordinates

genies uses XZ projective coordinates: a point P is represented as (X : Z) where x = X/Z. the y-coordinate is not stored — it is not needed for scalar multiplication or isogeny computation.

differential addition (given P, Q, and P-Q):
```
U = (X_P - Z_P)(X_Q + Z_Q)
V = (X_P + Z_P)(X_Q - Z_Q)
X_{P+Q} = Z_{P-Q} * (U + V)^2
Z_{P+Q} = X_{P-Q} * (U - V)^2
```

doubling:
```
S = (X_P + Z_P)^2
D = (X_P - Z_P)^2
X_{2P} = S * D
Z_{2P} = (S - D) * (S + ((A+2)/4)(S - D))
```

cost: 5 F_q multiplications per differential addition step, 4 F_q multiplications per doubling. all operations use multi-limb arithmetic since q is 512 bits.

## see also

- [[isogenies]] — the maps between curves
- [[class-group]] — the algebraic structure that acts on the curves
- [[montgomery-ladder]] — constant-time scalar multiplication on Montgomery curves
