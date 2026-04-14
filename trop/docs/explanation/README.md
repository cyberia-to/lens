# tropical semiring arithmetic

an encyclopedia of the mathematics behind trop's (min, +) algebra — from first principles to applications. every concept is grounded in the semiring we implement: tropical addition is min, tropical multiplication is ordinary addition, and shortest paths are matrix products.

## foundations

- [[tropical-semiring]] — field axioms fail, semiring axioms hold, idempotency changes everything
- [[matrix-algebra]] — tropical matrices turn shortest paths into linear algebra

## algorithms

- [[kleene-star]] — the Kleene star is to (min, +) what matrix inversion is to (+, x), and Floyd-Warshall computes it
- [[verification]] — LP duality provides optimality certificates, verified over F_p

## context

- [[applications]] — shortest paths, scheduling, phylogenetics, auction theory, string matching
- [[f2-fp-trop]] — three arithmetics (F₂, F_p, (min, +)), when to use which, how they compose in a proof system
