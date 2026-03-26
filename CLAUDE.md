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
field/        F_p lens (expander-graph codes, Brakedown construction, impl: nebu)
binary/       F₂ lens (binary Reed-Solomon, Binius construction, impl: kuro)
ring/         R_q lens (NTT-batched expander codes, impl: jali)
tropical/     (min,+) lens (witness-verify via field lens, impl: trop)
isogeny/      F_q lens (expander codes over isogeny field, impl: genies)
```

## companion repos

### arithmetics (lens depends on — one per algebra)

| repo | path | algebra |
|------|------|---------|
| nebu | `~/git/nebu/` | F_p scalar + extensions |
| kuro | `~/git/kuro/` | F₂ binary tower |
| jali | `~/git/jali/` | R_q polynomial ring |
| trop | `~/git/trop/` | (min,+) tropical semiring |
| genies | `~/git/genies/` | F_q isogeny group action |

### architecture (the five-layer stack)

```
hemera (identity)  → lens (commitment)  → nox (execution) → zheng (verification) → bbg (state)
~/git/hemera/        ~/git/lens/ (this)    ~/git/nox/         ~/git/zheng/            ~/git/bbg/
```

## do not touch zones

- `Cargo.toml` dependency versions — discuss before changing
- `reference/` — canonical spec, change there first then propagate
- Lens trait interface — requires cross-repo coordination (nox + zheng + bbg)
