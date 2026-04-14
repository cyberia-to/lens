# Montgomery ladder: constant-time scalar multiplication

scalar multiplication — computing [k]P for a scalar k and a point P — is the inner loop of both isogeny computation and point finding in genies. the Montgomery ladder performs this operation in constant time using only x-coordinates, making it ideal for side-channel-resistant isogeny arithmetic.

## the problem

genies needs to compute [k]P frequently:
- **cofactor multiplication**: [(q+1)/l_i] * P to extract a point of order l_i
- **kernel enumeration**: [2]P, [3]P, ..., [(l-1)/2]P for Velu's formulas
- **point validation**: [l]P = O to verify order

the scalar k can be up to 512 bits (for cofactor multiplication). the computation must be constant-time because the point P and curve E_A may depend on the secret key.

## the ladder algorithm

Montgomery's insight: on a Montgomery curve, you can compute x([k]P) using only x-coordinates, without ever computing y-coordinates. the algorithm maintains two points that always differ by P:

```
input: scalar k = (k_{n-1}, ..., k_1, k_0) in binary, point P = (X_P : Z_P)

R_0 = (1 : 0)       // point at infinity
R_1 = (X_P : Z_P)   // the point P
// invariant: R_1 - R_0 = P at every step

for i = n-1 down to 0:
    if k_i = 0:
        R_1 = R_0 + R_1    (differential addition, difference = P)
        R_0 = [2] R_0      (doubling)
    else:
        R_0 = R_0 + R_1    (differential addition, difference = P)
        R_1 = [2] R_1      (doubling)

output: R_0 = (X : Z) representing x([k]P) = X/Z
```

each step performs one differential addition and one doubling — the same operations regardless of the bit value. only the assignment of results to R_0 and R_1 changes.

## constant-time conditional swap

the branch "if k_i = 0 ... else ..." is implemented as a constant-time conditional swap:

```
// before the step:
cswap(k_i, R_0, R_1)    // swap R_0 and R_1 if k_i = 1

// always do:
R_1 = R_0 + R_1
R_0 = [2] R_0

// after the step:
cswap(k_i, R_0, R_1)    // swap back
```

the cswap operation is implemented using bitwise masking:

```
mask = -(k_i as u64)    // all-ones if k_i = 1, all-zeros if k_i = 0
for each limb j:
    t = mask & (R_0[j] ^ R_1[j])
    R_0[j] ^= t
    R_1[j] ^= t
```

no branches. no data-dependent memory accesses. the CPU executes exactly the same instructions regardless of the secret bit value. timing, power consumption, and cache behavior are identical for k_i = 0 and k_i = 1.

## XZ differential addition

the differential addition formulas for Montgomery curves in XZ coordinates (given P, Q, and x(P-Q)):

```
U = (X_P - Z_P) * (X_Q + Z_Q)
V = (X_P + Z_P) * (X_Q - Z_Q)
add = (U + V)^2
sub = (U - V)^2
X_{P+Q} = Z_{P-Q} * add
Z_{P+Q} = X_{P-Q} * sub
```

cost: 4 multiplications, 2 squarings, 6 additions in F_q. since squaring costs the same as multiplication for multi-limb arithmetic, this is effectively 6 F_q multiplications.

## XZ doubling

```
sum  = X_P + Z_P
diff = X_P - Z_P
S = sum^2
D = diff^2
E = S - D
X_{2P} = S * D
Z_{2P} = E * (D + a24 * E)
```

where a24 = (A + 2) / 4, precomputed once per curve. cost: 3 multiplications, 2 squarings, 3 additions. effectively 5 F_q multiplications.

## total cost per scalar multiplication

the Montgomery ladder processes one bit per step. for a b-bit scalar:
- b steps, each with one addition (6 muls) and one doubling (5 muls)
- total: 11b F_q multiplications

for cofactor multiplication with a ~512-bit scalar: ~5,600 F_q multiplications. for kernel enumeration with scalars up to (l-1)/2 ~ 293: ~3,200 F_q multiplications per l = 587.

## why not double-and-add

the textbook double-and-add algorithm has the same asymptotic cost but is NOT constant-time: it performs a different operation (add vs skip) depending on the bit value. even "constant-time" variants using dummy additions leak through microarchitectural side channels.

the Montgomery ladder is inherently constant-time: every step does the same two operations (addition + doubling). the only secret-dependent operation is the conditional swap, which is a bitwise operation with no branching.

## why not full coordinates

working with (X : Y : Z) projective coordinates would allow standard (non-differential) addition, enabling windowed methods (NAF, sliding window) that are faster for scalar multiplication. genies uses XZ-only coordinates because:

1. **memory**: XZ points are 2 limb arrays vs 3. in the isogeny loop, this matters.
2. **simplicity**: differential addition avoids the y-coordinate recovery step.
3. **compatibility**: Velu's formulas only need x-coordinates of kernel points.
4. **constant-time**: windowed methods use lookup tables with secret-dependent indices, which leak through cache timing. the ladder avoids this entirely.

when the y-coordinate IS needed (e.g., for point encoding), it can be recovered from x and the curve equation at the cost of one F_q square root.

## see also

- [[supersingular-curves]] — the curves on which the ladder operates
- [[isogenies]] — the computation that uses scalar multiplication as a subroutine
- [[csidh-prime]] — why the scalars are 512 bits
