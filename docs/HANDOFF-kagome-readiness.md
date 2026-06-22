# Minuet — Agent Hand-Off: Kagome Readiness Sprint

**Project:** Minuet — holographic memory toolkit built on amari-holographic
**Branch:** `docs/kagome-readiness-handoff` (rebased onto `main` — the AGPL v0.3.0 base)
**Date:** 2026-06-19 (revised after branch-topology verification; repo re-verified same day — see §0)
**Status:** Decisions locked (§10). **WS 0 COMPLETE (2026-06-19): `develop` synced ← `origin/main`, redundant stranded branch deleted (§0). Next action = WS 1 (amari floor `^0.15` → `^0.23`).**
**Purpose:** Prepare Minuet for integration with [Kagome](../Kagome) (the microwave-optical
back-end) — restore capability lost in the v0.3.0 tech-debt release, fix the amari
dependency seam, and close the gaps a physical back-end exposes.
**Predecessor:** [`HANDOFF.md`](../HANDOFF.md) (v0.3.0 relicense/tech-debt handoff, now on `main`).

> **Read first.** The v0.3.0 relicense **is on `main`** (PR #5, merged before gitflow was
> clamped down) — `main` is the AGPL/conformance base. Only `develop` is stale and needs
> syncing from `main` (§4). All four open decisions are resolved (§10).

---

## 0. State snapshot + WS 0 result (2026-06-19)

### WS 0 — COMPLETE

- **`develop` synced ← `origin/main`.** Merge commit `5665040` on `develop` (parents
  `5aade1e` + `18b3320`); `develop` is now **content-identical to `origin/main`**
  (`git diff origin/main develop` is empty). Pushed to origin (`5aade1e..5665040`).
- **Conformance verified on `develop`:** `LICENSE` = AGPL header, `license = "AGPL-3.0-only"`,
  `// SPDX-License-Identifier: AGPL-3.0-only` on `src/lib.rs`, `LICENSE-COMMERCIAL` present,
  `LICENSE-MIT`/`LICENSE-APACHE` removed; `Cargo.toml` version `0.3.0`.
- **Redundant stranded branch deleted.** `feature/relicense-ia-conformance-0.3.0` removed
  from origin (it added nothing over `origin/main`: empty `git diff --diff-filter=A`).
  **Recovery SHA `756dab0`** (recreate with `git branch <name> 756dab0`).
- **Regression state on `develop`:** the deleted retrieval files (`attribution.rs`,
  `resonator.rs`, `temperature.rs`) are gone (only `direct.rs`, `mod.rs`,
  `resonator_retriever.rs` remain) — i.e. the v0.3.0 amari-fusion removal is present, as
  expected. Restoring it is WS 2–4.
- **Green on `develop`:** `cargo test --features "parallel,serde,async,optical"` →
  82 lib unit tests + 11 doc-tests (8 ignored) pass; `cargo fmt --check` and
  `cargo clippy --features "parallel,serde,async,optical" -- -D warnings` clean.
- **amari floor intentionally untouched** (`amari-holographic = "0.15"`, lockfile `0.15.1`) —
  the bump is WS 1.

### WS 1 — DONE (2026-06-19) + §3 premise corrected

- **amari floor bumped `^0.15` → `^0.23`** on `feature/amari-floor-0.23` (commit `0e8fce5`,
  **PR #10** against `develop`). Zero source changes; resolves `amari-holographic 0.23.0` +
  `amari-core 0.23.0`. Green: 82 lib unit tests + 11 doc-tests, fmt/clippy/doc clean across
  the full feature matrix.
- **rand skew materialized as a split tree** (Minuet `rand` 0.8 for `optical` / amari
  `rand` 0.10) that compiles cleanly — no rand types cross the boundary. Consolidation
  deferred.
- **§3 premise corrected (verified by byte-diffing cached 0.15.1 vs 0.23.0 sources):** the
  optical public surface (`OpticalRotorField`, `OpticalFieldAlgebra`, `GeometricLeeEncoder`,
  `LeeEncoderConfig`, `BinaryHologram`, `TropicalOpticalAlgebra` w/ `attractor_step`/
  `attractor_converge`), `Resonator<A: BindingAlgebra>`, and `ResonatorConfig` are
  **byte-identical 0.15.1↔0.23.0**. amari-holographic itself changed only 2 files
  (`optical/lee_encoder.rs`, `optical/rotor_field.rs`), both same LOC. The bump did **not**
  "unlock" those symbols — they were reachable all along. Its real payload is the
  transitive **amari-core 0.15→0.23** upgrade (6032→8125 LOC, 14→21 files, **7 new modules**).
- **Consequence for later workstreams:** WS 3 (`Attribution` over `BindingAlgebra`/
  `Resonator<A>`) and WS 5 (`optical_store` via `OpticalFieldAlgebra`) were **never actually
  blocked by the 0.15 floor** — the APIs were identical at 0.15.1. Their "depends on WS 1" in
  §8 is sequencing hygiene, not a hard floor dependency.

### Correction recorded (local vs canonical main)

The first draft of §0 treated **local `main` (`bcbba1b`)** as "main". The canonical
upstream is **`origin/main` = `18b3320`** (one commit ahead of `bcbba1b` — the
`.github/workflows/project.yml` auto-project workflow). Local `main` was stale; it was
fast-forwarded to `origin/main` as part of WS 0. When this doc says "main" below, it means
`origin/main`. The sync was clean precisely because `develop`'s only divergent commit
(`5aade1e`) adds a `project.yml` byte-identical to `origin/main`'s (`18b3320`).

### Where this handoff branch stands now

`docs/kagome-readiness-handoff` was built on `bcbba1b` and carries its own `project.yml`
commit (`132d9b0`, byte-identical to `18b3320`/`5aade1e`) plus the handoff/correction/
re-verify commits (`d57cc0c`, `2f9f9e5`, `e5eb3ef`). **It now rebases cleanly onto the
synced `develop`** — the duplicate `project.yml` dedupes and the doc commits replay on top.
Recommended before its PR: `git rebase develop` so it targets `develop` (normal gitflow).

### Unchanged facts (still valid going into WS 1)

- **amari floor** `0.15` → bump to `0.23` is **WS 1 (the next action)**.
- **`amari-holographic` 0.23.0 is published** (crates.io, 2026-05-24; not yanked) — WS 1
  target is real.
- **Test baseline:** 82 lib unit tests + 11 doc-tests (8 ignored) under `WORKING_FEATURES`;
  41 lib unit tests minimal. (Earlier "69-test suite" figure was inaccurate. The "11
  integration" phrasing in earlier drafts was also wrong — those are **doc-tests**, not
  integration tests. WS 2 raises the counts to 89 lib + 13 doc-tests once PR #11 merges.)
- **CI is strong** (`.github/workflows/ci.yml`, `RUSTFLAGS=-Dwarnings`): check / test / fmt /
  clippy / docs / examples / minimal / feature-matrix, nightly via `dtolnay/rust-toolchain`.
  ⇒ §6 / WS 6 largely pre-satisfied; remaining nits in §6.
- **Concrete drift to expect in WS 1:** amari 0.23 depends on `rand` / `rand_chacha`
  **0.10**; Minuet pins `rand` **0.8**. Plan split-rand-tree or bump (touches
  `MockOpticalHardware` and RNG-using code).

> **PR targeting note (updated).** `develop` is now the non-stale AGPL/conformance base.
> `feature/*` readiness PRs (WS 1+) target `develop` per IA gitflow. This doc branch should
> be rebased onto `develop` (see above) before its PR.
>
> **Other remote branches (out of WS-0 scope, not yet assessed):** `origin/feature/optical-backend`
> and `origin/refactor/toolkit-conversion` still exist. They were **not** part of WS 0 and
> are not deleted. Worth a staleness check in WS 6 / housekeeping.

### GPU acceleration — deferred to `amari-gpu` 0.25.0 (no Minuet Borsalino dep)

Assessed during the sprint (2026-06-21) whether Minuet should integrate
[`Borsalino`](../Borsalino) directly for GPU acceleration of the optical compute path
(WS 5). **Decision: no — defer to `amari-gpu` 0.25.0.** Minuet's GPU story flows
`amari-gpu` → `amari-holographic` → Minuet, the same layering as everything else Minuet
consumes; when 0.25.0 lands and Minuet bumps its amari floor, the WS 5 `optical_store`
compute path (bind/bundle via `OpticalFieldAlgebra`) gets GPU acceleration for free —
with no Minuet-side kernel code.

Evidence backing the deferral (primary-source):
- **Borsalino v0.2.1 is a generic GPU-dispatch layer**, not an algebra layer: `GpuBackend`
  trait, WGSL → Metal/Vulkan via `naga`, buffers/pipelines/dispatch/readback, plus a
  `verify` feature for GPU *safety* properties (karpal). It has **no holographic knowledge**
  (no `bind`/`bundle`/`similarity`/`OpticalRotorField`) — it sits *below* where algebra
  kernels live. Its own `docs/verification-integration.md` scopes it as "the unsafe
  boundary layer"; algebraic laws are amari/Schubert's job.
- **Borsalino's verification is not yet release-ready** — `dispatch_verified()`/`Proven<>`
  gates, Miri, Kani, and `amari-flynn` are all open per its `VERIFICATION_ROADMAP_SUPPLEMENT.md`,
  and its "real IA kernel (geometric product)" item is still strategic-tier.
- **`amari-gpu` (separate, `wgpu`) is not a dep of `amari-holographic` 0.23** (what Minuet
  pulls) — so Minuet currently reaches *no* GPU path; the dep graph has no Borsalino/amari-gpu
  leg to short-circuit.
- **`amari-gpu` 0.25.0 (announced)** is a sweeping update: `wgpu` to current **and**
  Borsalino integrated as a feature gate. That is the correct, upstream integration point.

So a Minuet-side Borsalino dep would (a) put holographic WGSL kernels in the wrong layer
(Minet instead of `amari-gpu`), (b) duplicate the 0.25.0 upstream work, (c) pull a
pre-release dependency, and (d) likely be re-architected when 0.25.0 lands. The one narrow
Minuet-resident reading considered — a `BorsalinoOpticalHardware` simulation backend
alongside `MockOpticalHardware` — was rejected for the same reasons (and because compute
acceleration of `bind`/`bundle` is `amari-gpu`'s job regardless). This reaffirms the
prior Kagome-session decision (2026-06-19): GPU kernels for the holographic algebra belong
in `amari-gpu` (lowest reusable layer); Borsalino stays out of Minuet's update.

---

## 1. Why this sprint

Kagome is the physical realization of Minuet's optical backend in the microwave regime.
It implements `minuet::optical::OpticalHardware` (the substrate-specific trait) and is, at
the algebra level, a **port** of `amari-holographic::optical` (swapping the DMD pixel grid
for a cavity/resonator-mode basis). See Kagome's
[`docs/research/SURVEY.md`](../Kagome/docs/research/SURVEY.md) for the full design synthesis.

Three things in current Minuet block or degrade that integration:

1. **Lost retrieval capability.** The v0.3.0 tech-debt release deleted the tropical-dual
   resonator, attribution tracking, and temperature-annealed cleanup — capability Kagome's
   resonator-cleanup operation depends on (§2).
2. **A stale amari seam.** Minuet pins `amari-holographic` at `^0.15`, eight releases
   behind the maintained `0.23` line (and the transitive `amari-core` upgrade that line
   carries). **Note (post-WS-1):** the optical-field algebra Kagome ports from is *not*
   gated by this floor — it was reachable at `0.15.1` already (§0/§3) — but staying eight
   releases behind is still tech debt worth closing (§3).
3. **A stubbed optical compute path.** `CheckpointedOpticalMemory::optical_store` is a
   deliberate no-op — exactly the compute core a physical backend accelerates (§5).

---

## 2. Issue 1 — The amari-fusion removal (the regression)

### What happened

The v0.3.0 tech-debt release (`756dab0` on `feature/relicense-ia-conformance-0.3.0`) deleted:

| File | Lines | What it provided |
|------|------:|------------------|
| `src/retrieval/resonator.rs` | 418 | Tropical-dual resonator cleanup, `HierarchicalResonator` |
| `src/retrieval/attribution.rs` | 357 | `Attribution` / `AttributionResult` / `RetrievalExplanation` |
| `src/retrieval/temperature.rs` | 360 | `Temperature` enum, `TemperatureSchedule` (linear/exp/cosine annealing) |
| `benches/{binding_throughput,capacity_scaling,parallel_ops,retrieval_latency}.rs` | — | Pre-0.3.0 benchmarks |
| `docs/GHRR-implementation-plan.md` | — | (belonged in amari-holographic; deletion fine) |

Total: ~1,135 lines of retrieval code plus benchmarks.

### Why it read as "dead" (and why it wasn't)

`resonator.rs` and `attribution.rs` imported `use amari_fusion::{holographic::Bindable, TropicalDualClifford};`.
But the v0.1.0 "toolkit conversion" refactor (`00c4914`) moved Minuet's main algebra to
`amari_holographic::ProductCliffordAlgebra`. So the deleted retrieval files sat on a
**parallel `TropicalDualClifford` path** that the main store no longer fed — they compiled
against an unpublished-at-the-time surface and looked unreachable. The v0.3.0 cleanup
treated that as dead code.

**It was not redundant.** `Attribution` (which stored item contributed most to a retrieval)
and `Temperature`/`TemperatureSchedule` (annealed cleanup) exist **nowhere else** in Minuet
or amari today. Only the *generic* resonator moved: `amari-holographic` now ships
`Resonator<A: BindingAlgebra>` (`amari-holographic/src/memory/resonator.rs`), and Minuet's
surviving `ResonatorRetriever` (`src/retrieval/resonator_retriever.rs`, ~190 lines)
delegates to it. So:

- **Resonator cleanup:** partially survived (generic form, via amari-holographic).
- **Attribution tracking:** **lost** — no replacement anywhere.
- **Temperature-annealed cleanup:** **lost** — pure Minuet code, deleted wholesale.

### Verify before restoring

`TropicalDualClifford` still lives in **`amari-fusion`** at `amari-fusion/src/types.rs:93`
(published **0.23.0** on crates.io), and `amari-holographic`'s own docs point to it
(`use amari_fusion::TropicalDualClifford;`). So the restore target is amari-fusion — the
RABBIT_HOLE report's "moved into amari-holographic proper" was only half-true.

### Decision: restore strategy — DECIDED (option B + experimental TDC path)

**Chosen: B — generic re-expression over `BindingAlgebra`, plus an optional experimental
`TropicalDualClifford` fusion path.** Specifically:

- **`temperature.rs`** — restore near-verbatim (pure Minuet, no fusion import).
- **`Attribution`** — re-express over `A: BindingAlgebra` (not `TropicalDualClifford`).
- **Resonator cleanup** — use amari-holographic's generic `Resonator<A>` (already in
  `ResonatorRetriever`) as the default path.
- **Experimental TDC path** — behind an *additive* feature (e.g. `tropical-dual`, pulling
  `amari-fusion`), restore the `TropicalDualClifford`-specific resonator API as an opt-in.
  Additive only — never removes the generic path — per IA feature conventions.

  > **Revised in WS 4b (2026-06-20).** The `tropical-dual`/`amari-fusion` path was
  > **dropped** after audit: `amari-fusion` 0.23's `TropicalDualClifford` reinitializes its
  > dual representation inside `bind`/`unbind`/`bundle` (they all end in `from_clifford()`,
  > which discards input duals), so duals do **not** propagate through holographic ops —
  > true gradient-through-bundling attribution is not achievable via it as-is. Instead, WS 4b
  > implemented **real forward-mode dual (gradient) attribution directly over
  > `BindingAlgebra`** (on amari-core, no `amari-fusion` dep). See PR #15 and §8 row 4b.
  > The pre-v0.3.0 `compute_gradient` was itself only a stub ("fall back to
  > similarity-based attribution"), so nothing of value was lost.

**Rationale.** One default algebra path (`BindingAlgebra`) keeps store and retrievers
coherent and substrate-agnostic (what Kagome needs), while the experimental feature
preserves the tropical-dual semantics that motivated the original code. A noisy microwave
backend makes annealed cleanup and attribution *more* valuable, not less — they directly
attack the phase-noise error budget — and Kagome should not have to take a tropical-dual
dependency to get cleanup.

---

## 3. Issue 2 — The amari seam (version skew)

### The skew

```toml
# Minuet Cargo.toml (both develop and v0.3.0 branch)
amari-holographic = { version = "0.15", features = ["serialize"] }
```

`amari-holographic` is published at **0.23.0** — **eight releases** ahead of Minuet's floor
(0.15 → 0.23). Minuet (and therefore Kagome, via its `minuet` dep) is locked to 0.15.x
(Kagome's `Cargo.lock` shows `amari-holographic 0.15.1`).

### What's unreachable through Minuet's floor — CORRECTED: nothing on the surface

> **Original premise withdrawn (2026-06-19, after WS 1).** The first draft asserted the
> optical-field algebra symbols below were present at 0.23.0 but absent at 0.15. A
> byte-level diff of the cached 0.15.1 vs 0.23.0 sources disproves this: **they are
> byte-identical.** The bump still stands (see §0 + §3 decision), but not for the reason
> given here.

The symbols the earlier draft listed — `amari-holographic::optical` (`OpticalRotorField`,
`OpticalFieldAlgebra` `bind`/`bundle`/`similarity`/`unbind`/`add_phase`, `GeometricLeeEncoder`
+ `LeeEncoderConfig`, `BinaryHologram`), `TropicalOpticalAlgebra` with `attractor_step`/
`attractor_converge`, and `Resonator<A: BindingAlgebra>` — are all **already present and
identical in amari-holographic 0.15.1**. The 0.15.1→0.23.0 delta is **2 in-place edits**
(`optical/lee_encoder.rs`, `optical/rotor_field.rs`, same LOC) — no new public surface, no
`Resonator`/retrieval drift. **Kagome was never actually locked out of the optical-field
algebra by Minuet's floor.**

What the bump *does* buy (the real reason to do it): the transitive **`amari-core`
0.15→0.23** upgrade — 6032→8125 LOC, 7 new core modules — plus currency (no longer 8
releases behind the maintained line). The `MemoryBackend`-adjacent surface in
[`IA-documents/Minuet/Physical-backend-implementation-pathways.md`](../IA-documents/Minuet/Physical-backend-implementation-pathways.md)
§13 should be re-checked against amari-core 0.23, where most movement occurred.

### Decision: how to close the seam — DECIDED (option A)

**Chosen: A — bump `amari-holographic` to `"0.23"` (the 0.23.x line).** ✅ **DONE (WS 1,
PR #10).** The decision stands, but the rationale is revised: the bump was pitched as
"highest-leverage" on the theory that it unlocks the optical-field algebra and de-risks
Kagome's port. That theory was **disproven by the WS 1 drift audit** (§0) — nothing on
amari-holographic's surface was unlocked. The bump's actual leverage is the **transitive
`amari-core` 0.15→0.23 upgrade** (7 new modules) and currency. It remains the right call
(zero source changes, all green) and a sensible sequencing prerequisite; it is just not the
capability-unblock the first draft claimed.

### Audit the drift — RESULT (post-WS-1)

`amari-holographic` 0.23.0 was confirmed published (crates.io, 2026-05-24, not yanked) and
the bump landed clean. **The predicted drift did not occur.** Diffing the cached 0.15.1 vs
0.23.0 sources across every symbol Minuet uses (`BindingAlgebra`, `ProductCliffordAlgebra`,
`Resonator`, `ResonatorConfig`, `HolographicMemory`, `AlgebraConfig`, `RetrievalResult`,
plus the `optical::*` set): **all byte-identical.** The `Resonator<A: BindingAlgebra>` the
earlier draft called newly-generic has been at `src/memory/resonator.rs:75` since 0.15.1.
Only 2 files changed (lee_encoder, rotor_field), in-place.

**rand skew — resolved as a split tree.** amari 0.23 depends on `rand`/`rand_chacha` **0.10**,
Minuet pins `rand` **0.8**. This compiled cleanly with **no source changes** — Minuet's
`optical` rand usage never exchanges rand types with amari, so a split rand tree (0.8 +
0.10) coexists without conflict. Consolidating to a single rand version is optional future
cleanup (would touch `MockOpticalHardware` + RNG-seeding code), deliberately **not** done in
WS 1. The 82-test (working-features) / 41-test (minimal) suite stayed green throughout — it
was the safety net and it held.

---

## 4. Issue 3 — Branch topology (the relicense is on `main`; `develop` is stale)

> Corrected after verifying the actual branch state. The first draft of this handoff
> assumed the relicense was stranded; it is not.

**The relicense is on `main`.** PR #5 (`4a40d9a` "Relicense to AGPL-3.0 & IA conformance
(v0.3.0)") was merged **directly to `main`** by an agent before gitflow was clamped down,
then released (`5e0068d`, tag `v0.3.0`, on crates.io). Verified on `origin/main`:

- `LICENSE` = AGPL (header "Minuet — Holographic Memory Systems"), `LICENSE-COMMERCIAL`
  present, `LICENSE-MIT` removed; SPDX headers on `src/lib.rs`; `Cargo.toml` version 0.3.0.
- The **retrieval-file deletion is on `main`** (only `direct.rs`, `mod.rs`,
  `resonator_retriever.rs` remain) — confirming the amari-fusion removal landed.
- All the v0.3.0 additions are on `main`: `HANDOFF.md`, `CONTRIBUTING.md`, `docs/ROADMAP.md`,
  `rust-toolchain.toml`, `benches/holographic_ops.rs`, `tests/integration/end_to_end.rs`.

**The stranded branch is redundant.** `feature/relicense-ia-conformance-0.3.0` (`756dab0`)
adds **nothing** not already on `main` (`git diff --diff-filter=A` is empty). It's an
abandoned duplicate of the work that landed via PR #5. **Safe to delete.**

**`develop` is stale.** It is **behind `main` by 7 commits** (the entire v0.3.0 line) and
**ahead by 1** (`5aade1e`, a CI auto-project-routing workflow that `main` doesn't have).
So `develop` is still effectively pre-relicense (MIT/Apache, still has the retrieval files)
— the opposite of what the first draft assumed.

### Decision: base sequencing — DECIDED

**Sync `develop` from `main`** (merge `main` → `develop`), preserving `5aade1e`; then delete
the redundant `feature/relicense-ia-conformance-0.3.0`. This is workstream 0 (§8) and the
base for every sprint PR. After the sync, `develop` is the AGPL/conformance baseline and
this handoff branch (rebased onto `main`) targets it cleanly.

**Why it matters for Kagome:** Kagome is AGPL-3.0-only; `main` already provides the
licensing-coherent base. The sync just gets `develop` onto that base so the readiness PRs
follow normal gitflow (`feature/*` → `develop`).

---

## 5. Issue 4 — The optical compute stub

`CheckpointedOpticalMemory::optical_store` is a deliberate no-op:

```rust
// src/optical/checkpoint.rs:525  (current develop)
#[allow(clippy::unused_self)]
#[allow(clippy::unnecessary_wraps)]
fn optical_store(&mut self, _key: &OpticalRotorField, _value: &OpticalRotorField)
    -> Result<(), MemoryError> {
    // In a full implementation, this would:
    // 1. Bind key with value to create memory trace
    // 2. Bundle with existing memory (superposition)
    // 3. Optionally display and measure for resonator cleanup
    // For now, we rely on the logical state for retrieval
    Ok(())
}
```

The public `store()` calls it, but retrieval falls back to the logical (symbolic) state.
This is the **compute accelerator path** — exactly what a physical backend (optical or
microwave) is for. It was correctly characterized as a deliberate "ship persistence first,
fill compute later" choice (per `HANDOFF.md`), not a bug.

**For Kagome readiness**, implement the compute path using `OpticalFieldAlgebra`
(**already reachable** — it was present at amari-holographic 0.15.1; the WS 1 bump added
currency, not capability, see §0): bind → bundle into the memory trace → display+measure
for cleanup. This is
**shared work with Kagome**: Kagome's `MicrowaveField` is the port of `OpticalRotorField`,
so the bind/bundle logic written here is reused there. **DECIDED:** implement behind the
existing `optical` feature with a software (`MockOpticalHardware`) reference, **in
coordination with Kagome**, so it's testable without physical hardware.

---

## 6. Other readiness points

- **CI — verified present and strong** (`.github/workflows/ci.yml`, re-checked 2026-06-19).
  `RUSTFLAGS=-Dwarnings` is set globally. Jobs: `check`, `test`, `fmt`
  (`cargo fmt --all -- --check`), `clippy` (`-- -D warnings`), `docs`
  (`cargo doc --no-deps`, `RUSTDOCFLAGS=-Dwarnings`), `examples`, `minimal`
  (`--no-default-features`), and a `features` matrix (`""`, `parallel`, `serde`, `async`).
  Toolchain is nightly via `dtolnay/rust-toolchain`, which honors `rust-toolchain.toml`.
  The `test`/`check`/`clippy`/`docs` jobs run with `WORKING_FEATURES = parallel,serde,async,optical`.
  **Remaining nits for WS 6:** (a) the `features` matrix does **not** isolate `optical`
  standalone (it's only exercised via the all-features jobs) — add an `optical` matrix leg;
  (b) `persistence` is excluded everywhere by design (RocksDB needs a C++ toolchain) — keep
  it excluded but document the requirement in the README. IA reference template:
  [`IA-documents/IA-rust-common/rust-ci-template.yml`](../IA-documents/IA-rust-common/rust-ci-template.yml).
- **`persistence` feature** requires a C++ toolchain (RocksDB) and is not tested in CI per
  `HANDOFF.md`. Document this in the README; ensure Kagome-facing examples don't depend on it.
- **Doc-test hygiene.** Several examples in `src/lib.rs` and `resonator_retriever.rs` are
  `rust,ignore` (e.g. the `ProductCliffordAlgebra` quick-start). When the amari floor bumps,
  revisit which can be un-ignored.
- **`optical` module doc references `LeeEncoderConfig`** — reachable after §3; the optical
  module docs and `CheckpointedOpticalMemory` should be audited against the real
  `GeometricLeeEncoder` API.

---

## 7. What Kagome needs from Minuet (integration surface)

So the implementing session knows the downstream contract:

1. **`minuet::optical::OpticalHardware` trait** — Kagome implements this as its
   microwave substrate-specific layer (`display` = drive resonators, `measure` = IQ
   demodulate, `quick_calibrate`/`full_calibrate` = characterize S-matrix). Already wired
   in Kagome's `src/minuet.rs`.
2. **A working `optical_store` compute path** (§5) — Kagome's value is accelerating this;
   if it stays a no-op, the integration is structural only.
3. **Resonator cleanup that's substrate-agnostic** (§2) — Kagome's cleanup runs on noisy
   microwave retrievals; it needs attribution + annealed temperature, not just the generic
   Resonator.
4. **A non-stale amari floor** (§3) — so Kagome doesn't have to bypass Minuet to reach the
   optical-field algebra it ports.
5. **AGPL licensing coherence** (§4) — Kagome is AGPL-3.0-only.

---

## 8. Recommended sprint plan (workstream sequencing)

Ordered by dependency; each item is a PR against `develop`. **WS 0 done:** `develop` is
the synced AGPL/conformance base; this handoff branch is rebased onto `develop` (§0).

| # | Workstream | Depends on | Approx scope |
|---|-----------|------------|--------------|
| 0 | **Sync `develop` ← `main` + delete redundant stranded branch** (§4) | — | **✅ DONE (2026-06-19):** `develop` = `5665040`, content-identical to `origin/main` (`5aade1e`+`18b3320`); `feature/relicense-ia-conformance-0.3.0` deleted (recovery `756dab0`); pushed to origin. |
| 1 | **Bump `amari-holographic` floor `^0.15` → `^0.23`** (§3-A) | 0 | Cargo.toml + adapt to API drift (expect rand 0.8→0.10 skew); 82-test safety net |
| 2 | **Restore `temperature.rs`** (§2-B) | 0 | near-verbatim; pure Minuet, no new dep |
| 3 | **Re-express `Attribution` over `BindingAlgebra`** (§2-B) | 1 | port `attribution.rs` from `TropicalDualClifford` to generic `A` |
| 4 | **Wire annealed temperature into `ResonatorRetriever`** (§2-B) | 2,3 | close the loop: annealed cleanup as a retriever option |
| 4b | **Real dual-number gradient attribution** (§2, revised) | 1,4 | **DONE (PR #15):** real forward-mode dual attribution on amari-core — `attrib_i = ⟨rᵢ,result⟩/⟨result,result⟩`, exact sum-to-1 against the pure-sum superposition. **No `amari-fusion` dep, no `tropical-dual` feature** (audit showed TDC duals don't survive `bind`/`bundle`; original `compute_gradient` was a stub). |
| 5 | **Implement `optical_store` compute path** (§5) | 1 | bind+bundle via `OpticalFieldAlgebra`; Mock-hardware reference impl; coordinate with Kagome |
| 6 | **CI + doc hygiene pass + branch cleanup** (§6) | 0 | CI already strong (§6) — add `optical` matrix leg, document `persistence` C++ need in README, un-ignore doc-tests where the amari bump allows; **staleness-check `origin/feature/optical-backend` and `origin/refactor/toolkit-conversion`** (survived WS 0 unassessed) — delete if redundant, recover-via-SHA if not |

Items 1–4 are the "amari seam + fusion restore" core. Item 5 is the Kagome-shared compute
path. Items 0 and 6 are housekeeping that should bookend the sprint.

> **Sequencing caveat (post-WS-1 audit):** the "depends on WS 1" edges on rows 3 and 5 are
> soft, not hard. The APIs WS 3 (`Resonator<A: BindingAlgebra>`) and WS 5
> (`OpticalFieldAlgebra`) need were identical at amari-holographic 0.15.1 — they were never
> floor-blocked (§0). Keeping WS 1 first is good hygiene (one diff base), but 3/5 could in
> principle have proceeded on 0.15.1.

---

## 9. Verification checklist (before declaring the sprint done)

- [x] `develop` synced from `main`: AGPL-3.0-only, SPDX header on `src/lib.rs`, redundant
      stranded branch deleted (§4). `develop`=`5665040` ≡ `origin/main`; verified green
      (82 lib unit + 11 doc-tests, fmt/clippy clean).
- [~] `amari-holographic` resolves to 0.23.x in the lockfile (§3) — **done in PR #10
      (WS 1), pending merge to `develop`.**
- [ ] `cargo test --all-features` green; test count ≥ 82 (current CI working-features
      baseline; 41 minimal — §0) and *increased* by restored retrieval coverage (§2).
- [ ] `Attribution` and `TemperatureSchedule` are reachable from the public API and have
      doc-tests (§2).
- [ ] `optical_store` performs a real bind+bundle verifiable via `MockOpticalHardware`
      (§5).
- [ ] `cargo clippy --features "parallel,serde,async,optical" -- -D warnings` and
      `cargo doc --no-deps` clean (CI deliberately excludes `persistence`, which needs a
      C++ toolchain — §6).
- [ ] Kagome's `--features minuet` build still passes against the updated Minuet (§7).

---

## 10. Decisions (locked)

1. **Base sequencing (§4):** sync `develop` ← `main`; delete redundant stranded branch. ✅
2. **amari seam (§3):** bump `amari-holographic` to `"0.23"` as the first code PR. ✅
   *(Decision holds; rationale revised — see §0/§3: the bump brings the `amari-core`
   transitive upgrade + currency, not the optical-symbol "unlock" the draft claimed.)*
3. **Fusion restore (§2):** generic re-expression over `BindingAlgebra` (option B). ✅
   *(Revised in WS 4b: the planned optional `tropical-dual` feature / `amari-fusion` dep
   was **dropped** — audit showed TDC duals don't propagate through `bind`/`bundle`, and
   the original `compute_gradient` was a stub. Instead, WS 4b delivers real forward-mode
   gradient attribution on amari-core. See §0 "GPU acceleration" note, §2, §8 row 4b, PR #15.)*
4. **`optical_store` (§5):** implement behind the `optical` feature with a
   `MockOpticalHardware` reference impl, **in coordination with Kagome** (whose
   `MicrowaveField` ports `OpticalRotorField`). ✅

---

## 11. Pointers

- Kagome survey (the design this sprint unblocks):
  [`../Kagome/docs/research/SURVEY.md`](../Kagome/docs/research/SURVEY.md) — esp. §10
  (the amari optical-field algebra as Kagome's template) and §8 (the ADR backlog).
- Substrate survey (Minuet's place in the backend taxonomy):
  [`../IA-documents/Minuet/Physical-backend-implementation-pathways.md`](../IA-documents/Minuet/Physical-backend-implementation-pathways.md)
  — §13 (unified backend abstraction), §15.1 (microwave PoC roadmap).
- Amari-integration analysis (the 5 opportunities, verified against amari 0.23.0):
  [`../IA-documents/RESEARCH_REPORTS_archive/AMARI_INTEGRATION_MINUET_2026-02-27_20260227_150000.md`](../IA-documents/RESEARCH_REPORTS_archive/AMARI_INTEGRATION_MINUET_2026-02-27_20260227_150000.md).
- v0.3.0 handoff (predecessor): [`../HANDOFF.md`](../HANDOFF.md) (now on `main`).
