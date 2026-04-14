# isogeny group action arithmetic

an encyclopedia of the mathematics behind genies — supersingular isogenies, class group actions, and the CSIDH construction. every concept connects back to the central operation: the commutative group action of cl(O) on supersingular elliptic curves over F_q.

## foundations

- [[supersingular-curves]] — what makes a curve supersingular, Montgomery form, point arithmetic, the isogeny graph
- [[isogenies]] — morphisms between curves, Velu's formulas, kernel polynomials, composition

## the action

- [[class-group]] — the ideal class group cl(O), commutativity, why it enables non-interactive protocols
- [[csidh-prime]] — why the CSIDH prime exists, why Goldilocks cannot work, smooth factorization of q+1

## implementation

- [[montgomery-ladder]] — constant-time scalar multiplication, XZ coordinates, side-channel resistance
- [[verification]] — folding 512-bit F_q arithmetic into 64-bit Goldilocks for zheng proofs
