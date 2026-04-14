# agent collaboration

principles for working with AI coding agents across any project. this page is the bootstrap entry point — read it and the four foundational documents to have complete development context:

- [[cyber/engineering]] — pipeline contracts, dual-stream optimization, verification dimensions
- [[cyber/quality]] — 12 review passes, severity tiers, audit protocol
- [[cyber/projects]] — repo layout, namespace conventions, git workflow
- [[cyber/documentation]] — Diataxis framework, reference vs docs, spec before code

---

# lens — polynomial commitment

## what lens is

lens is the commitment layer for [[cyber]]. five polynomial commitment backends — one per algebra. the layer between [[hemera]] (identity) and [[nox]] (execution).

a lens makes computation verifiable: commit to a polynomial, prove evaluations, verify without seeing the polynomial. each algebra sees through its own optic.

## architecture

```
hemera → lens → nox → zheng → bbg
```

lens is consumed by three components:
- nox: noun identity (Lens.commit wraps polynomial noun)
- zheng: proof commitment (SuperSpartan queries lens)
- bbg: state commitment (BBG_root uses Lens.commit)

## five lenses

```
scalar-field/        Brakedown    (expander-graph codes over F_p, impl: nebu)
binary-tower/        Binius       (binary Reed-Solomon over F₂, impl: kuro)
polynomial-ring/     Ikat         (NTT-batched, structure IS the pattern, impl: jali)
tropical-semiring/   Assayer      (witness-verify via dual certificate, impl: trop)
isogeny-curves/      Porphyry     (expander codes over deep field F_q, impl: genies)
```

## workspace crates

### arithmetics (subcrates — one per algebra)

| crate | path | crates.io | algebra |
|-------|------|-----------|---------|
| nebu | `nebu/rs/` | cyb-nebu | F_p scalar + extensions |
| kuro | `kuro/rs/` | cyb-kuro | F₂ binary tower |
| jali | `jali/rs/` | cyb-jali | R_q polynomial ring |
| trop | `trop/rs/` | cyb-trop | (min,+) tropical semiring |
| genies | `genies/rs/` | cyb-genies | F_q isogeny group action |

each also has `wgsl/` (GPU backend) and `cli/` (command-line tool) subcrates.

### external repos (the five-layer stack)

```
hemera (identity)  → lens (commitment)  → nox (execution) → zheng (verification) → bbg (state)
~/git/hemera/        this repo             ~/git/nox/         ~/git/zheng/            ~/git/bbg/
```

## do not touch zones

- `Cargo.toml` dependency versions — discuss before changing
- `specs/` — canonical spec, change there first then propagate
- Lens trait interface — requires cross-repo coordination (nox + zheng + bbg)
