# encoding

serialization formats for genies types.

## F_q element

an element x in F_q is serialized as 64 bytes, little-endian.

```
representation: x = sum(x_i * 2^(64*i)) for i = 0..7
encoding:       [x_0_LE || x_1_LE || ... || x_7_LE]    (8 * 8 = 64 bytes)
```

each x_i is a 64-bit limb stored in little-endian byte order. the value x must satisfy 0 <= x < q.

**canonical form**: on deserialization, reject if the decoded value >= q. there is exactly one valid encoding per F_q element.

## curve serialization

a Montgomery curve E_A: y^2 = x^3 + Ax^2 + x is determined by the single coefficient A in F_q.

```
curve encoding: encode(A) as F_q element    (64 bytes)
```

the base curve E_0 has A = 0. the shared secret in DH is the encoded A' of the result curve (or equivalently its j-invariant).

## point serialization

a point P = (x, y) on E_A is serialized as x-coordinate plus sign bit:

```
point encoding: [x_bytes || sign_byte]    (64 + 1 = 65 bytes)
    sign_byte = 0x00 if y is the "smaller" square root (y < q/2)
    sign_byte = 0x01 if y is the "larger" square root (y >= q/2)
```

the point at infinity O is encoded as 65 zero bytes.

**recovery**: given x and sign bit, compute s = x^3 + Ax^2 + x, check that s is a quadratic residue, compute y = sqrt(s), and select the root matching the sign bit.

## secret key encoding

a secret key is an exponent vector (e_1, ..., e_n) where each e_i in {-m, ..., m}.

```
encoding: [e_1 + m, e_2 + m, ..., e_n + m]    (n bytes, unsigned)
```

each exponent is shifted by m to make it non-negative, then stored as a single byte. for CSIDH-512: n = 74, m = 5, so each byte is in {0, ..., 10}, and the encoding is 74 bytes.

## j-invariant encoding

the j-invariant j(E_A) is an F_q element. encoded identically to any F_q element (64 bytes, little-endian).

two curves are isomorphic iff they have the same j-invariant, making this a canonical curve identifier.

## byte order convention

all multi-byte integers in genies use **little-endian** byte order, consistent with the broader cyber stack convention (nebu, kuro).
