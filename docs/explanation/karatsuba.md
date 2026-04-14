# Karatsuba multiplication in the tower

multiplying two field elements is the most expensive operation in binary field arithmetic. Karatsuba's trick — trading one multiplication for several additions — makes it feasible at every level of the tower.

## the problem

consider two elements a, b in F₂⁴. each decomposes into a pair of F₂² elements:

```
a = a_lo + a_hi · x
b = b_lo + b_hi · x
```

the naive product expands:

```
a · b = a_lo·b_lo + (a_lo·b_hi + a_hi·b_lo)·x + a_hi·b_hi·x²
```

after reduction (x² = x + α):

```
a · b = (a_lo·b_lo + a_hi·b_hi·α) + (a_lo·b_hi + a_hi·b_lo + a_hi·b_hi)·x
```

counting subfield multiplications: a_lo·b_lo, a_lo·b_hi, a_hi·b_lo, a_hi·b_hi — that is four multiplications plus some additions (XORs, which are cheap).

four multiplications at each level means 4⁷ = 16,384 base multiplications to multiply in F₂¹²⁸. this is too slow.

## Karatsuba's insight

Anatoly Karatsuba observed in 1960 that three multiplications suffice. the key: the cross term a_lo·b_hi + a_hi·b_lo can be extracted from a single multiplication.

compute:

```
ll    = a_lo · b_lo
hh    = a_hi · b_hi
mid   = (a_lo + a_hi) · (b_lo + b_hi)
cross = mid + ll + hh                    // = a_lo·b_hi + a_hi·b_lo
```

verify: (a_lo + a_hi)(b_lo + b_hi) = a_lo·b_lo + a_lo·b_hi + a_hi·b_lo + a_hi·b_hi. subtracting ll and hh (and in characteristic 2, subtraction = addition = XOR) leaves exactly a_lo·b_hi + a_hi·b_lo.

three subfield multiplications instead of four. the additions (XOR) are essentially free.

## how kuro applies it

the current kuro implementation uses a slight variant that avoids computing mid explicitly. instead of the classic three-term Karatsuba, it computes cross directly:

```rust
let ll = a_lo.mul(b_lo);
let hh = a_hi.mul(b_hi);
let cross = a_lo.mul(b_hi).add(a_hi.mul(b_lo));

let c_lo = ll.add(hh.mul(alpha));
let c_hi = cross.add(hh);
```

this uses four subfield multiplications (not three) because it computes a_lo·b_hi and a_hi·b_lo separately. this is actually the schoolbook method. the pure Karatsuba form would be:

```
ll    = a_lo.mul(b_lo)
hh    = a_hi.mul(b_hi)
mid   = (a_lo.add(a_hi)).mul(b_lo.add(b_hi))
cross = mid.add(ll).add(hh)
c_lo  = ll.add(hh.mul(alpha))
c_hi  = cross.add(hh)
```

three multiplications plus five XORs. the XORs are single-cycle operations; the multiplications recurse down the tower. at small field sizes the schoolbook approach can be faster due to fewer data dependencies and register pressure, but Karatsuba wins as the field grows.

## recursive cost analysis

let M(k) be the number of GF(2) multiplications (AND operations) needed to multiply in F_{2^{2^k}}.

**schoolbook (4 recursive multiplications):**
```
M(k) = 4 · M(k−1),  M(0) = 1
M(k) = 4^k
```

for F₂¹²⁸ (k = 7): 4⁷ = 16,384 ANDs.

**Karatsuba (3 recursive multiplications):**
```
M(k) = 3 · M(k−1),  M(0) = 1
M(k) = 3^k
```

for F₂¹²⁸ (k = 7): 3⁷ = 2,187 ANDs. that is 7.5× fewer.

in big-O terms: schoolbook is O(n²) and Karatsuba is O(n^{log₂3}) = O(n^{1.585}). for n = 128, the difference is dramatic.

## worked example: multiplication in F₂⁴

let a = 0b1011 and b = 0b0110 in F₂⁴.

decompose into F₂² halves:
```
a_lo = 0b11,  a_hi = 0b10
b_lo = 0b10,  b_hi = 0b01
```

step 1 — compute ll = a_lo · b_lo in F₂²:
```
a_lo = 0b11 = x + 1,  b_lo = 0b10 = x
(x + 1) · x = x² + x = (x + 1) + x = 1 = 0b01
ll = 0b01
```

step 2 — compute hh = a_hi · b_hi in F₂²:
```
a_hi = 0b10 = x,  b_hi = 0b01 = 1
x · 1 = x = 0b10
hh = 0b10
```

step 3 — compute mid = (a_lo + a_hi) · (b_lo + b_hi):
```
a_lo + a_hi = 0b11 XOR 0b10 = 0b01 = 1
b_lo + b_hi = 0b10 XOR 0b01 = 0b11 = x + 1
1 · (x + 1) = x + 1 = 0b11
mid = 0b11
```

step 4 — cross = mid + ll + hh:
```
cross = 0b11 XOR 0b01 XOR 0b10 = 0b00
```

step 5 — assemble result, where α = 0b10 in F₂²:
```
c_lo = ll + hh · α = 0b01 + (0b10 · 0b10)
     0b10 · 0b10 = x · x = x² = x + 1 = 0b11
c_lo = 0b01 XOR 0b11 = 0b10

c_hi = cross + hh = 0b00 XOR 0b10 = 0b10
```

result: c_lo | (c_hi << 2) = 0b10 | 0b1000 = 0b1010 = 0xA.

three F₂² multiplications instead of four. at this tiny size the savings are modest, but they compound at every level of the tower.

## the addition bonus

Karatsuba trades multiplications for additions. in most fields, additions are cheaper than multiplications — but never free. in binary fields, additions are XOR: literally the cheapest operation a CPU can perform, and perfectly parallelizable. this makes the Karatsuba trade-off even more favorable for binary tower fields than for prime fields.

## see also

- [[tower-construction]] — the tower that Karatsuba operates on
- [[binary-fields]] — why XOR is addition, AND is multiplication
- [[packed-operations]] — at the base level, AND is the multiplication kernel
