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
  82 lib + 11 integration (8 ignored) pass; `cargo fmt --check` and
  `cargo clippy --features "parallel,serde,async,optical" -- -D warnings` clean.
- **amari floor intentionally untouched** (`amari-holographic = "0.15"`, lockfile `0.15.1`) —
  the bump is WS 1.

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
- **Test baseline:** 82 lib + 11 integration (8 ignored) under `WORKING_FEATURES`; 41
  minimal. (Earlier "69-test suite" figure was inaccurate.)
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
2. **A stale amari seam.** Minuet pins `amari-holographic` at `^0.15`; the mature
   optical-field algebra Kagome ports from is at `0.23.0` and unreachable through Minuet's
   floor (§3).
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

### What's unreachable through Minuet's floor

Verified present in amari-holographic **0.23.0** but not in 0.15:

- `amari-holographic::optical` — `OpticalRotorField`, `OpticalFieldAlgebra`
  (`bind`/`bundle`/`similarity`/`unbind`/`add_phase`), `GeometricLeeEncoder` +
  `LeeEncoderConfig`, `BinaryHologram` — **the reference algebra Kagome ports from**.
- `TropicalOpticalAlgebra` with `attractor_step` / `attractor_converge` — the resonator
  cleanup primitive in tropical-dual flavour.
- The `MemoryBackend`-adjacent surface described in
  [`IA-documents/Minuet/Physical-backend-implementation-pathways.md`](../IA-documents/Minuet/Physical-backend-implementation-pathways.md) §13.

### Decision: how to close the seam — DECIDED (option A)

**Chosen: A — bump `amari-holographic` to `"0.23"` (the 0.23.x line).** This is the
single highest-leverage change in the sprint and the first code PR (after the `develop`
sync, §4/§8). Both the fusion restore and Kagome's port depend on it; doing it first
isolates 0.15→0.23 API-drift breakage against the existing test suite (82 tests under CI's
working-features set; see §0 and "Audit the drift" below).

### Audit the drift

`amari-holographic` 0.23.0 is confirmed published (crates.io, 2026-05-24, not yanked), so
this target is real. Before merging the bump, run a focused diff of the symbols Minuet uses
(`BindingAlgebra`, `ProductCliffordAlgebra`, `Resonator`, `ResonatorConfig`,
`HolographicMemory`, `AlgebraConfig`, `RetrievalResult`) across 0.15 → 0.23. Expect
renames/signature changes in the `Resonator` and retrieval surface (those are exactly the
areas that gained the generic `Resonator<A>`).

**Known concrete drift (verified from the 0.23.0 manifest):** amari 0.23 depends on
`rand` / `rand_chacha` **0.10**, while Minuet pins `rand` **0.8** — budget for a split rand
tree or for bumping Minuet to rand 0.10 (touches `MockOpticalHardware` and any RNG-using
code). The existing test suite (82 tests under CI's working-features set, 41 minimal — §0)
is the safety net.

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

**For Kagome readiness**, implement the compute path using `OpticalFieldAlgebra` (reachable
once §3 lands): bind → bundle into the memory trace → display+measure for cleanup. This is
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

Ordered by dependency; each item is a PR against `develop`. **Prerequisite:** WS 0 must
sync `develop` ← `main` first — this handoff branch currently sits on `main`, not
`develop` (see §0).

| # | Workstream | Depends on | Approx scope |
|---|-----------|------------|--------------|
| 0 | **Sync `develop` ← `main` + delete redundant stranded branch** (§4) | — | **✅ DONE (2026-06-19):** `develop` = `5665040`, content-identical to `origin/main` (`5aade1e`+`18b3320`); `feature/relicense-ia-conformance-0.3.0` deleted (recovery `756dab0`); pushed to origin. |
| 1 | **Bump `amari-holographic` floor `^0.15` → `^0.23`** (§3-A) | 0 | Cargo.toml + adapt to API drift (expect rand 0.8→0.10 skew); 82-test safety net |
| 2 | **Restore `temperature.rs`** (§2-B) | 0 | near-verbatim; pure Minuet, no new dep |
| 3 | **Re-express `Attribution` over `BindingAlgebra`** (§2-B) | 1 | port `attribution.rs` from `TropicalDualClifford` to generic `A` |
| 4 | **Wire annealed temperature into `ResonatorRetriever`** (§2-B) | 2,3 | close the loop: annealed cleanup as a retriever option |
| 4b | **Experimental `tropical-dual` feature** (§2) | 1,4 | additive feature pulling `amari-fusion`; restore the `TropicalDualClifford` resonator API as opt-in; default path unchanged |
| 5 | **Implement `optical_store` compute path** (§5) | 1 | bind+bundle via `OpticalFieldAlgebra`; Mock-hardware reference impl; coordinate with Kagome |
| 6 | **CI + doc hygiene pass** (§6) | 0 | CI already strong (§6) — add `optical` matrix leg, document `persistence` C++ need in README; un-ignore doc-tests where the amari bump allows |

Items 1–4 are the "amari seam + fusion restore" core. Item 5 is the Kagome-shared compute
path. Items 0 and 6 are housekeeping that should bookend the sprint.

---

## 9. Verification checklist (before declaring the sprint done)

- [x] `develop` synced from `main`: AGPL-3.0-only, SPDX header on `src/lib.rs`, redundant
      stranded branch deleted (§4). `develop`=`5665040` ≡ `origin/main`; verified green
      (82+11 tests, fmt/clippy clean).
- [ ] `amari-holographic` resolves to 0.23.x in the lockfile (§3).
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
3. **Fusion restore (§2):** generic re-expression over `BindingAlgebra` (option B), **plus**
   an optional experimental `tropical-dual` feature restoring the `TropicalDualClifford`
   fusion API as an additive opt-in. ✅
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
