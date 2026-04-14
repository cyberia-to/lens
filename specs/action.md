# class group action algorithm

the core primitive of genies: compute [a] * E for an ideal class [a] and a supersingular curve E.

## input

- ideal class [a] represented as exponent vector (e_1, e_2, ..., e_n) with e_i in {-m, ..., m}
- curve E given by Montgomery coefficient A in F_q (so E_A: y^2 = x^3 + Ax^2 + x)

## output

- curve E' = [a] * E, given by Montgomery coefficient A' in F_q

## algorithm (naive CSIDH)

```
function action(exponents, A):
    for i = 1 to n:
        while exponents[i] != 0:
            // find a point of order l_i on E_A (or its twist)
            repeat:
                sample random x in F_q
                compute s = x^3 + A*x^2 + x
                if exponents[i] > 0 and is_square(s):
                    P = (x, sqrt(s))          // point on E_A
                elif exponents[i] < 0 and not is_square(s):
                    P = (x, sqrt(-s))         // point on twist
                else: continue
                Q = [(q+1)/l_i] * P           // cofactor multiply
            until Q != O

            // compute l_i-isogeny
            A = velu(A, Q, l_i)

            // decrement exponent
            if exponents[i] > 0: exponents[i] -= 1
            else: exponents[i] += 1

    return A
```

## positive vs negative exponents

- e_i > 0: apply the l_i-isogeny on the curve E_A itself (find point on E_A)
- e_i < 0: apply the l_i-isogeny on the quadratic twist of E_A (find point on twist, which corresponds to the dual isogeny direction)

the twist of E_A over F_q has #E'(F_q) = q + 1 - t when #E(F_q) = q + 1 + t. for supersingular curves with t = 0, the twist has the same cardinality, but the isogeny walks in the opposite direction in the class group.

## cost model

per action with CSIDH-512 parameters:

| component | cost |
|-----------|------|
| primes | n = 74, l_i from 3 to 587 |
| max exponent | m = 5, so up to 5 isogenies per prime |
| avg point finding | ~2 attempts per prime (prob 1/2) |
| avg isogeny cost | O(l_i) F_q muls, avg l ~ 150 |
| total F_q muls | ~ 74 * 5 * 150 = ~55,500 |
| F_q mul cost | ~8x64-bit schoolbook = ~64 limb muls |
| wall time (est.) | ~50-100 ms on modern CPU |

## constant-time (dCTIDH)

the naive algorithm leaks timing information: different exponent vectors take different amounts of time (variable number of isogeny steps, variable point-finding attempts).

dCTIDH (dummy-free constant-time CSIDH) fixes this:

1. **fixed number of steps per prime**: always perform exactly B_i steps for prime l_i, regardless of the actual exponent. unused steps compute dummy isogenies that are discarded.
2. **constant-time point finding**: use deterministic point generation (Elligator) instead of rejection sampling.
3. **uniform key sampling**: rejection-sample exponent vectors to achieve uniform distribution in the class group.

constant-time is enforced down to the F_q arithmetic level: no branches on secret data, no variable-time multiplication, no early-exit.

## DH convenience wrapper

```
function dh(secret, peer_curve):
    return action(secret, peer_curve)
```

one line. the commutativity of the class group action gives:

```
dh(a, dh(b, E_0)) = action(a, action(b, E_0)) = action(b, action(a, E_0)) = dh(b, dh(a, E_0))
```

## batch action

when computing multiple actions sharing the same curve (e.g., during key generation for multiple recipients), batch optimizations apply:

- shared point-finding: reuse random points across actions for the same curve
- shared cofactor multiplication: compute [(q+1)/l_i] * P once, reuse across batch
- amortized inversion: Montgomery batch inversion for projective-to-affine conversions
