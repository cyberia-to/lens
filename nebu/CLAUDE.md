# agent collaboration

principles for working with AI coding agents across any project. this page is the bootstrap entry point — read it and the four foundational documents to have complete development context:

- [[cyber/engineering]] — pipeline contracts, dual-stream optimization, verification dimensions
- [[cyber/quality]] — 12 review passes, severity tiers, audit protocol
- [[cyber/projects]] — repo layout, namespace conventions, git workflow
- [[cyber/documentation]] — Diataxis framework, reference vs docs, spec before code

## auditor mindset

the project is supervised by an engineer with 30 years of experience.
deception does not work. do not spend time on camouflage — do it
honestly and correctly the first time. every attempt to hide a problem
behind formatting, substitute numerator for denominator, or show
"progress" where there is none will be caught and require rework.
one time correctly is cheaper than five times beautifully.

## honesty

never fake results. never fill empty columns with duplicate data to
make things look complete. if a system produces nothing — show nothing.
a dash is more honest than a copied number.

the purpose of every metric, column, and indicator is to reflect
reality. never substitute appearance of progress for actual progress.
never generate placeholder data to fill a gap. if you catch yourself
making something "look right" instead of "be right" — stop and
delete it.

## literal interpretation

when the user says something, they mean it literally. do not
reinterpret. do not find the closest thing you think they might mean.
do not iterate on your interpretation 13 times.

known failure mode: the user says "show real numbers" and the agent
reformats display labels, adds tags, restructures output — everything
except showing the actual data the user asked for. this is the
masquerading instinct — optimizing for "looks correct" instead of
"is correct."

rules:

1. if the user asks to show data, show the raw value from the source
   before any fallback, gating, or cleanup
2. if you are unsure what the user means, ask once. do not guess and
   iterate
3. if your first instinct is to format/present/clean — stop. ask
   "what is the raw data the user has not seen yet?" show that first
4. never hide failure behind technically-accurate-but-misleading numbers
5. the user knows what they are saying. trust their words over your
   interpretation of their intent

## chain of verification

for non-trivial decisions affecting correctness:

1. initial answer
2. 3-5 verification questions that would expose errors
3. answer each independently — check codebase, re-read docs
4. revised answer incorporating corrections

skip for trivial tasks.

## estimation model

estimate work in sessions and pomodoros, not months.

- pomodoro = 30 minutes of focused work
- session = 3 focused hours (6 pomodoros)

LLM-assisted development compresses traditional timelines — a
"2-month project" might be 6-8 sessions. plan in reality, not
in inherited assumptions.

## agent memory

all plans and design documents persist in the project repo, not in
ephemeral agent storage. plans go to `<repo-root>/.claude/plans/`.

rules:

1. read what is already there before writing
2. before presenting a plan for approval, write it to a file first.
   the user reviews the file in their editor, not the chat
3. every plan the user signs off on gets committed to the repo.
   rejected plans get deleted
4. compress old entries when files grow stale — density over volume

## compaction survival

when context compacts, preserve: modified file paths, failing test
names, current task intent, and uncommitted work state.

## parallel agents

split parallel agents by non-overlapping file scopes. never let two
agents edit the same file. partition by directory. use subagents for
codebase exploration. keep main context clean for implementation.

## writing style

state what something is directly. never use "this is not X, it is Y"
formulations. never define by negation.

---

# engineering patterns

architectural patterns that apply across projects.

## pipeline contract

```
Stage_0 → Stage_1 → Stage_2 → ... → Stage_N → Output
```

output of stage N must be valid input for stage N+1. when modifying
a stage, verify both its input and its output still connect.

define a clear pipeline boundary — the point where the compiler/tool
ends and the runtime begins. everything before the boundary is the
build system. everything after is execution.

## dual-stream optimization

two independent optimization streams run in parallel:

1. hand-written baseline: write from first principles — algorithm +
   target machine, never from compiler output. ask "what is the
   minimum instruction sequence for this operation?" not "how can I
   improve what the compiler emitted?"

2. automated pipeline: improve codegen to approach hand baselines.
   every baseline with ratio > 1.5x is an optimization target.

the streams must stay independent. hand baselines set the floor —
the automated pipeline races toward it. when the pipeline catches up,
push the baseline lower. neither stream is dogma; both improve
continuously.

the benchmark suite is the scoreboard. regressions in either
direction are bugs.

## self-verification

every commit:

- type check / lint — zero warnings
- test suite — all tests pass
- benchmark — no regressions vs baselines
- audit — formal properties still hold (where applicable)
- if anything fails, fix before reporting done

## multi-dimensional verification

every function that targets provable execution is verified across
independent dimensions:

| dimension | source | role |
|-----------|--------|------|
| reference | ground truth implementation | generates inputs, computes expected outputs |
| automated | default build pipeline | standard compilation |
| manual | hand-optimized baseline | expert floor |
| learned | ML/neural optimizer | exploration of solution space |

four metrics compared across all dimensions:

1. correctness — output matches reference on all test inputs
2. execution speed — cycle count or runtime
3. proving time — proof generation cost (where applicable)
4. verification time — proof checking cost (where applicable)

slow code is a bug. incorrect code is a soundness hole.

## companion repo pattern

when a project splits into a compiler/tool and a runtime/executor:

- keep them in separate repos with path dependencies
- use repo-qualified paths when referencing files across repos
- after editing the upstream repo, rebuild downstream too
- patches over forks: fetch upstream, apply diff, vendor result

## sync rules

when a concept must stay synchronized across multiple locations
(spec, type checker, code generator, cost model), document the
list of locations explicitly. every change to one location requires
updating all others in the same commit.

---

# quality control

a methodology for reviewing code in projects where correctness matters.
every line of code may end up in a proof circuit, a financial system,
or an autonomous agent. quality means soundness.

## file size limit

no single source file should exceed 500 lines. if it does, split it
into submodules. entry point files (re-exports only) are the only
exception.

## review passes

invoke passes by number. on full audit — run all passes in parallel
using agents, persist results, prepare a fix plan before applying.

### pass 1: determinism

- no floating point in deterministic paths
- no hash map iteration (non-deterministic order)
- no system clock, no randomness without explicit seed
- serialization is canonical — single valid encoding per value
- cross-platform: same input → same output, always

"find any source of non-determinism in this code."

### pass 2: bounded locality

- every function's read-set is O(k)-bounded
- no hidden global state
- graph walks have explicit depth/hop limits
- state updates touch only declared write-set
- local change cannot trigger unbounded cascade

"what is the maximum read-set and write-set?"

### pass 3: domain arithmetic correctness

- all reductions correct — no overflow before reduce
- multiplication uses widening where needed
- inverse/division handles zero explicitly
- batch operations: individual vs batch results match
- edge values correct: 0, 1, max-1, max

"check edge cases. does reduction overflow?"

### pass 4: crypto hygiene

- no secret-dependent branching (constant-time)
- no secret data in error messages, logs, or debug output
- zeroize sensitive memory on drop
- hash domain separation — unique prefix per use
- constraints: neither under-constrained nor over-constrained

"is there any path where secret material leaks?"

### pass 5: type safety and invariants

- newtypes for distinct domains (UserId != PostId)
- states encoded in types (Unverified vs Verified)
- unsafe blocks have safety comments
- no unwrap on fallible paths
- invalid state construction prevented by type system

"can a caller construct an invalid state?"

### pass 6: error handling and degradation

- every error type is meaningful
- errors propagate with context
- no panic in library code
- resource cleanup on all error paths
- partial failure does not corrupt shared state

"what happens when this fails halfway through?"

### pass 7: adversarial input

- all external inputs validated before processing
- sizes, lengths, indices bounds-checked
- no allocation proportional to untrusted input without cap
- malformed input rejected before expensive computation

"what is the cheapest input an attacker can craft for maximum damage?"

### pass 8: architecture and composability

- single responsibility per module
- dependencies point inward (domain ← application ← infra)
- traits/interfaces define boundaries
- no circular dependencies
- public API is minimal

"can I replace this implementation without touching callers?"

### pass 9: readability and naming

- names match domain terminology
- functions do what their name says — no hidden side effects
- comments explain why, not what
- magic numbers are named constants with units
- code reads top-down

"can someone reading only this file understand what it does and why?"

### pass 10: compactness and elimination

- no dead code, no commented-out blocks
- no premature abstraction — one impl does not need a trait
- no duplicate logic
- no unnecessary allocations
- "what can I delete?" before "what should I add?"

"what can be removed without changing behavior?"

### pass 11: performance and scalability

- hot path is allocation-free
- no O(n^2) without justification and n-bound
- batch operations for anything called in loops
- cache-friendly access patterns
- profiled, not guessed

"what is the complexity at 10^9 items? where does it break first?"

### pass 12: testability

- pure functions where possible
- side effects injected (trait objects, closures)
- property-based tests for invariants
- edge case tests: empty, one, max, overflow, malicious
- test names describe the property, not the method

"what property should always hold? write a property test for it."

## severity tiers

| tier | passes | when |
|------|--------|------|
| every commit | 1, 5, 6, 9 | determinism, types, errors, readability |
| every PR | + 2, 7, 8, 10 | locality, adversarial, architecture, compactness |
| every release | + 3, 4, 11, 12 | crypto, arithmetic, performance, full test coverage |

## audit protocol

1. launch parallel agents partitioned by module scope (no overlapping files)
2. each agent runs assigned passes, writes findings to a results directory
3. main session reads findings, summarizes, prepares a fix plan
4. user confirms the fix plan before any changes are applied
5. fixes applied as atomic commits. stale findings cleaned up

---

# project structure

conventions for organizing repositories.

## repo layout

```
project/
├── src/                 implementation source code
├── specs/           canonical specification (source of truth)
│   ├── language.md      core language/API spec
│   ├── grammar.md       formal grammar (if applicable)
│   ├── ir.md            internal representations
│   ├── quality.md       review methodology
│   └── props/           design proposals (draft → accepted → implemented)
├── docs/                documentation (Diataxis framework)
│   ├── tutorials/       learning by building
│   ├── guides/          task-oriented how-tos
│   └── explanation/     conceptual understanding
├── .claude/             agent state (not ephemeral)
│   ├── plans/           design decisions and implementation plans
│   ├── audits/          audit logs and summaries
│   └── other/           performance reports, analysis, findings
├── CLAUDE.md            agent instructions (project-specific)
├── LICENSE.md           license (links to canonical source)
└── README.md            project entry point
```

## namespace conventions

four namespaces partition functionality:

| namespace | purpose |
|-----------|---------|
| vm.* | intrinsics — lowest-level primitives |
| std.* | libraries — reusable standard modules |
| os.* | portable runtime — platform abstraction |
| os.\<platform\>.* | platform-specific implementations |

source code mirrors namespaces as directories.

## specs/ vs docs/

specs/ is specification — defines what the system does. austere,
complete, organized by system structure. this is the source of truth.
when code and reference disagree, fix reference first, then propagate.

docs/ is documentation — teaches, guides, explains. organized by the
reader's needs (Diataxis). references spec but does not duplicate it.

## design proposals

`specs/props/` holds proposals for changes not yet committed to
the spec. each proposal has status frontmatter (draft, accepted,
rejected, implemented). proposals document desire before commitment.
accepted proposals migrate to the relevant specs/ file.
rejected proposals stay for rationale.

## agent state

`.claude/plans/` persists across conversations. plans are committed
to the repo so the user can review them in their editor. budget:
1000 lines total across `.claude/`. merge or delete weak entries
before adding new ones.

## configuration

project configuration lives in a manifest file at the root
(Cargo.toml, package.json, trident.toml, etc.). target-specific
configuration lives alongside the target code, not in the root.

## do not touch zones

every project has files that should not be modified without explicit
discussion: dependency manifests, canonical reference structure,
target configurations, license. document these in CLAUDE.md.

## git workflow

- atomic commits — one logical change per commit
- conventional prefixes: feat:, fix:, refactor:, docs:, test:, chore:
- feature branches for all work, PRs to merge
- branch naming: feat/, fix/, refactor/, docs/, test/, chore/
- test names describe the property, not the method

## companion repos

when a system spans multiple repos, document the relationship in
CLAUDE.md: what each repo does, how they depend on each other,
how to rebuild after changes, and how to reference files across repos.
use repo-qualified paths (e.g. `trident/src/` vs `trisha/src/`).

---

# documentation methodology

how to organize documentation across projects.

## diataxis framework

all documentation follows [Diataxis](https://diataxis.fr) — four types
organized by two axes.

|  | learning | working |
|---|---|---|
| practical | tutorials | how-to guides |
| theoretical | explanation | reference |

### tutorials (learning-oriented)

teach by doing. the reader follows steps and builds something.
the tutorial controls the experience. no choices, no alternatives.
success is guaranteed if the reader follows along.

### how-to guides (task-oriented)

solve a specific problem. the reader already knows what they want
to do. steps are direct. no teaching, no background. assumes
competence.

### reference (information-oriented)

describe the machinery. types, functions, APIs, configuration.
austere and complete. organized by the structure of the code, not
by the reader's journey.

### explanation (understanding-oriented)

illuminate concepts. why things work the way they do. background,
context, design decisions. not tied to a specific task.

## directory layout

```
docs/
├── tutorials/       learning by building
├── guides/          task-oriented how-tos
├── explanation/     conceptual understanding
└── README.md        index with links to all four quadrants

specs/           canonical specification (separate from docs/)
├── language.md      types, operators, builtins
├── grammar.md       formal grammar
├── ir.md            intermediate representation
├── props/           design proposals (not yet spec)
│   └── README.md    proposal lifecycle: draft → accepted → rejected → implemented
└── quality.md       review methodology
```

## source of truth

`specs/` is canonical. if specs/ and code disagree, resolve
in specs/ first, then propagate to code. if implementation reveals
the reference is wrong or incomplete, update the reference to match
reality.

specs/ is not documentation — it is specification. docs/ explains
and teaches. specs/ defines.

## design proposals

proposals for future changes live in `specs/props/`. each proposal
is a standalone markdown file with status frontmatter:

| status | meaning |
|--------|---------|
| draft | idea captured, open for discussion |
| accepted | approved — ready to implement and move to spec |
| rejected | decided against, kept for rationale |
| implemented | done — migrated to the relevant spec file |

proposals are not spec. they document desire before commitment.

## spec before code

write the reference entry before writing the implementation. the spec
is the contract. the code fulfills it. if you cannot specify what a
feature does before building it, you do not understand it yet.

---

## knowledge graph

this repo is a subgraph of `~/git/cyber/`. files with YAML
frontmatter (tags, crystal-type, crystal-domain, stake) are
nodes in the cyber knowledge graph. preserve frontmatter in
all files that have it — it is not decoration.

---

# nebu — Goldilocks field library

## what nebu is

nebu is the Goldilocks prime field $\mathbb{F}_p$ where $p = 2^{64} - 2^{32} + 1$.
it provides field arithmetic as a standalone primitive: add, sub, mul, inv, eq, lt,
plus NTT (Number Theoretic Transform) over roots of unity.

## companion repos

| repo | path | role |
|------|------|------|
| nebu | `~/git/nebu/` | Goldilocks field arithmetic (this repo) |
| hemera | `~/git/hemera/` | hash function (Poseidon2), depends on nebu |
| nox | `~/git/nox/` | VM, depends on nebu + hemera |
| zheng | `~/git/zheng/` | proof system (SuperSpartan + WHIR) |
| mudra | `~/git/mudra/` | crypto primitives (KEM, dCTIDH, TFHE, threshold) |
| bbg | `~/git/bbg/` | authenticated state (Big Badass Graph) |
| trident | `~/git/trident/` | language, compiles to nox |
| cyber | `~/git/cyber/` | knowledge graph (parent subgraph) |

## do not touch zones

- `Cargo.toml` dependency versions — discuss before changing
- `specs/` — canonical spec, change there first then propagate
