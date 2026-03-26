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
nebu/         F_p lens (expander-graph codes, Brakedown construction)
kuro/         F₂ lens (binary Reed-Solomon, Binius construction)
jali/         R_q lens (NTT-batched expander codes)
trop/         (min,+) lens (witness-verify via nebu lens)
genies/       F_q lens (expander codes over isogeny field)
```

## companion repos

| repo | path | role |
|------|------|------|
| nebu | `~/git/nebu/` | Goldilocks field arithmetic |
| hemera | `~/git/hemera/` | hash function (commitment binding) |
| kuro | `~/git/kuro/` | F₂ tower arithmetic |
| jali | `~/git/jali/` | R_q polynomial ring |
| trop | `~/git/trop/` | tropical semiring |
| genies | `~/git/genies/` | isogeny group action |
| lens | `~/git/lens/` | polynomial commitment (this repo) |
| nox | `~/git/nox/` | VM (consumes lens for noun identity) |
| zheng | `~/git/zheng/` | proof system (consumes lens for proof commitment) |
| bbg | `~/git/bbg/` | authenticated state (consumes lens for state root) |

## do not touch zones

- `Cargo.toml` dependency versions — discuss before changing
- `reference/` — canonical spec, change there first then propagate
- Lens trait interface — requires cross-repo coordination (nox + zheng + bbg)
