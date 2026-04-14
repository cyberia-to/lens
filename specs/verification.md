# verification

how genies arithmetic folds into zheng proofs.

## the folding challenge

genies operates over F_q (512-bit CSIDH prime). zheng operates over F_p (Goldilocks, 64-bit). to prove correctness of an isogeny computation, F_q arithmetic must be emulated as F_p constraints.

## F_q element representation in F_p

an F_q element (512-bit) is represented as 8 Goldilocks limbs:

```
x in F_q  ->  (x_0, x_1, ..., x_7)  where x = sum(x_i * 2^(64*i))
```

each x_i is a 64-bit value fitting in one Goldilocks field element (p = 2^64 - 2^32 + 1 > 2^63).

## F_q operation costs in F_p

| F_q operation | F_p cost | method |
|---------------|----------|--------|
| add | 8 F_p adds + carries | limb-wise with carry propagation |
| sub | 8 F_p subs + borrows | limb-wise with borrow propagation |
| mul | ~64 F_p muls + reduction | schoolbook on 8 limbs, Barrett reduce |
| inv | ~512 F_q muls | Fermat via addition chain |
| comparison | 8 F_p comparisons | limb-wise, high to low |

## single isogeny verification

an l-isogeny phi: E -> E' with kernel <P> is correct if:

1. P is on E: P.y^2 = P.x^3 + P.x (curve equation check)
2. P has order l: [l]P = O (scalar multiplication check)
3. E' matches Velu output (formula verification)
4. kernel polynomial divides the l-division polynomial

each check decomposes into F_q operations, each F_q operation into ~64 F_p constraints.

## full action verification

the class group action [a] * E composes multiple l-isogenies. verification checks the chain:

```
E = E_0 -> E_1 -> ... -> E_k = E'
```

where each E_i -> E_{i+1} is a valid l_i-isogeny. the verifier confirms:
- each step is a valid isogeny (kernel point check + Velu formula)
- composition yields the claimed output curve
- start curve matches the public input

## proof structure

```
genies execution (F_q)
  | produces witness: isogeny chain + intermediate curves
  v
fold into F_p constraints
  | F_q element -> 8 Goldilocks limbs
  v
zheng proof (F_p)
  | verifies folded constraints
  v
Goldilocks accumulator
```

the verifier never touches F_q directly. genies provides the folding arithmetic; zheng provides the proof system.

## cost estimate

per isogeny step with prime l:
- point finding: O(log q) F_q muls ~ O(log q * 64) F_p constraints
- Velu computation: O(l) F_q muls ~ O(l * 64) F_p constraints
- total per action: sum over all primes ~ O(n * avg(l) * 64) F_p constraints

for CSIDH-512: n = 74, avg(l) ~ 150, so ~700k F_p constraints per action. expensive but acceptable for infrequent operations.
