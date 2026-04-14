# inversion specification

computing a^{-1} in GF(2^n) for each tower level. two approaches: Fermat exponentiation and tower-recursive norm-based inversion.

## Fermat's little theorem

in GF(2^n), every nonzero element satisfies a^{2^n - 1} = 1. therefore:

```
a^{-1} = a^{2^n - 2}
```

the exponent 2^n - 2 has a binary representation of n-1 ones followed by a zero:

```
2^n - 2 = (1111...10)_2    (n-1 ones, then 0)
```

### naive square-and-multiply

```
inv(a):
  t = a
  for i in (n-2) down to 1:
    t = t²
    t = t * a
  t = t²                   // final squaring (bit 0 of exponent is 0)
  return t
```

cost: (n-1) squarings + (n-2) multiplications. for n = 128: 127 squarings + 126 multiplications.

## addition chain optimization

the exponent 2^n - 2 = 2 * (2^{n-1} - 1). computing a^{2^k - 1} has a doubling structure:

```
a^{2^1 - 1}  = a
a^{2^2 - 1}  = (a^{2^1 - 1})^{2^1} * a^{2^1 - 1}
a^{2^4 - 1}  = (a^{2^2 - 1})^{2^2} * a^{2^2 - 1}
a^{2^8 - 1}  = (a^{2^4 - 1})^{2^4} * a^{2^4 - 1}
...
a^{2^{2^k} - 1} = (a^{2^{2^{k-1}} - 1})^{2^{2^{k-1}}} * a^{2^{2^{k-1}} - 1}
```

then a^{-1} = (a^{2^n - 1})^{...} adjusted. the chain for each tower level:

### GF(2^2) -- F₂²

```
exponent = 2^2 - 2 = 2
a^{-1} = a²
```

cost: 1 squaring.

### GF(2^4) -- F₂⁴

```
e1 = a                        // a^{2^1 - 1}
e2 = e1^{2^1} * e1            // a^{2^2 - 1} = a^3
a^{-1} = e2^{2^1} * e1        // a^{2^2 - 2} * a^0 ...
       = (a^3)^2 = a^6        // but 2^4 - 2 = 14 = 1110_2
```

full chain for 2^4 - 2 = 14:

```
e1 = a                         // a^1
e2 = e1^2 * e1                 // a^3
e4 = e2^4 * e2                 // a^15... no, use:
t  = a^2                       // a^2
t  = t * a                     // a^3
t  = t^4                       // a^12
t  = t * a^2                   // a^14
```

cost: 3 squarings + 2 multiplications.

### GF(2^128) -- F₂¹²⁸

```
e1   = a                                // a^(2^1 - 1)
e2   = e1^(2^1) * e1                    // a^(2^2 - 1)       1 sq + 1 mul
e4   = e2^(2^2) * e2                    // a^(2^4 - 1)       2 sq + 1 mul
e8   = e4^(2^4) * e4                    // a^(2^8 - 1)       4 sq + 1 mul
e16  = e8^(2^8) * e8                    // a^(2^16 - 1)      8 sq + 1 mul
e32  = e16^(2^16) * e16                 // a^(2^32 - 1)     16 sq + 1 mul
e64  = e32^(2^32) * e32                 // a^(2^64 - 1)     32 sq + 1 mul
e128 = e64^(2^64) * e64                 // a^(2^128 - 1)    64 sq + 1 mul

a^{-1} = e128 * a^{-1}...
```

more precisely: a^{-1} = a^{2^128 - 2} = (a^{2^127 - 1})^2. so:

```
e127 = e64^(2^63) * ...       // build a^(2^127 - 1) from the chain
a^{-1} = e127^2               // one final squaring
```

the chain for 2^127 - 1:

```
e1   = a                               1 sq,  0 mul  (total: 0 sq, 0 mul)
e2   = e1^2 * e1                                      (total: 1 sq, 1 mul)
e4   = e2^(2^2) * e2                                  (total: 3 sq, 2 mul)
e8   = e4^(2^4) * e4                                  (total: 7 sq, 3 mul)
e16  = e8^(2^8) * e8                                  (total: 15 sq, 4 mul)
e32  = e16^(2^16) * e16                               (total: 31 sq, 5 mul)
e64  = e32^(2^32) * e32                               (total: 63 sq, 6 mul)
e127 = e64^(2^63) * e64^{...}                         (build from subchains)
```

total for 2^128 - 2: 127 squarings + 7 multiplications.

## tower-recursive inversion

the tower structure enables a recursive formula that reduces inversion in F_{2^{2k}} to inversion in F_{2^k}:

```
inv(a) where a = a_lo + a_hi * x in F_{2^{2k}}:
  // compute norm: N(a) = a_lo * (a_lo + a_hi) + a_hi² * alpha
  d = a_lo * (a_lo + a_hi) + sq(a_hi) * alpha     // d in F_{2^k}
  d_inv = inv(d)                                    // recursive call

  // a^{-1} = (a_lo + a_hi + a_hi * x) * d^{-1}
  result_lo = (a_lo + a_hi) * d_inv
  result_hi = a_hi * d_inv
  return result_lo + result_hi * x
```

### cost recurrence

let I(k) = cost of inversion in F_{2^{2^k}}, M(k) = cost of multiplication:

```
I(0) = 0                          (F₂: trivial)
I(k) = I(k-1) + 3*M(k-1) + S(k-1) + 2*A(k-1)
```

where S(k) is squaring cost and A(k) is addition cost (XOR, negligible).

| level | field | muls in subfield | inv in subfield | total subfield ops |
|-------|-------|-----------------|-----------------|-------------------|
| 1 | F₂² | 3 F₂ muls | 0 | 3 AND + 2 XOR |
| 2 | F₂⁴ | 3 F₂² muls | 1 F₂² inv | 3 F₂² mul + 1 F₂² inv |
| 3 | F₂⁸ | 3 F₂⁴ muls | 1 F₂⁴ inv | 3 F₂⁴ mul + 1 F₂⁴ inv |
| 7 | F₂¹²⁸ | 3 F₂⁶⁴ muls | 1 F₂⁶⁴ inv | 3 F₂⁶⁴ mul + 1 F₂⁶⁴ inv |

the recursion bottoms out at F₂ where inv(1) = 1 (and inv(0) is undefined).

## zero handling

zero has no inverse. the implementation must either:
- assert/panic on zero input (debug builds)
- return an arbitrary value and let the caller guarantee nonzero input (release builds)

see [batch](batch.md) for inverting many elements with zero handling.

## see also

- [field](field.md) -- operation definitions
- [batch](batch.md) -- amortized inversion of N elements
- [vectors](vectors.md) -- test vectors for inversion
