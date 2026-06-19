# Minuet — Agent Hand-Off: Kagome Readiness Sprint

**Project:** Minuet — holographic memory toolkit built on amari-holographic
**Branch:** `docs/kagome-readiness-handoff` (off `develop`)
**Date:** 2026-06-19
**Purpose:** Prepare Minuet for integration with [Kagome](../Kagome) (the microwave-optical
back-end) — restore capability lost in the v0.3.0 tech-debt release, fix the amari
dependency seam, and close the gaps a physical back-end exposes.
**Predecessor:** [`HANDOFF.md`](../HANDOFF.md) (v0.3.0 relicense/tech-debt handoff, on
`feature/relicense-ia-conformance-0.3.0`).

> **Read first.** This handoff assumes the v0.3.0 relicense branch exists but is **not
> merged** to `develop`/`main` (see §4). Decide the sequencing in §6 before starting any
> code work — it determines your base branch.

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

### Decision: restore strategy (⚠ pick before coding)

| Option | What | Trade |
|--------|------|-------|
| **A — Verbatim restore** | Re-add `amari-fusion` dep, revive the 3 files as-is | Re-introduces a parallel `TropicalDualClifford` code path divergent from the `ProductCliffordAlgebra` store; fastest, but doubles the algebra surface and the divergence that caused the "dead code" call in the first place. |
| **B — Re-express generically (recommended)** | Restore `temperature.rs` near-verbatim (it's pure Minuet, no fusion import). Re-express `Attribution` over `A: BindingAlgebra`. Replace the tropical-dual resonator with amari-holographic's generic `Resonator<A>` (already in `ResonatorRetriever`). Keep `amari-fusion` as an *optional* path for tropical-dual semantics, not the default. | More work upfront; one algebra path; aligns with amari-holographic's direction; attribution/temperature become substrate-agnostic (good for Kagome). |
| **C — Defer attribution/temperature** | Restore only what Kagome's first milestone needs (resonator cleanup is already generic). | Least work; loses attribution/temperature longer; those are exactly the retrieval-quality features a noisy physical backend needs. |

**Recommendation: B.** A noisy microwave backend makes annealed cleanup and attribution
*more* valuable, not less — they directly attack the phase-noise error budget. And Kagome
should not have to take a tropical-dual dependency to get cleanup.

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

### Decision: how to close the seam (⚠ pick before coding)

| Option | What | Trade |
|--------|------|-------|
| **A — Bump Minuet's floor to `^0.23` (recommended)** | One-line Cargo.toml change + adapt to any 0.15→0.23 API drift | Reaches the mature algebra ecosystem-wide; unblocks Kagome directly; exposes any drift Minuet must absorb. |
| **B — Leave Minuet at 0.15; Kagome takes amari-holographic 0.23 as a direct dep** | No Minuet change | Kagome reaches the algebra, but Minuet stays stale and the ecosystem stays forked on amari versions. |

**Recommendation: A.** The floor bump is the single highest-leverage change in this sprint.
Do it as its **own PR** first (before the fusion restore), because both the restore and
Kagome's port depend on it and it isolates API-drift breakage.

### Audit the drift

Before merging the bump, run a focused diff of the symbols Minuet uses
(`BindingAlgebra`, `ProductCliffordAlgebra`, `Resonator`, `ResonatorConfig`,
`HolographicMemory`, `AlgebraConfig`, `RetrievalResult`) across 0.15 → 0.23. Expect
renames/signature changes in the `Resonator` and retrieval surface (those are exactly the
areas that gained the generic `Resonator<A>`). The 69-test suite is the safety net.

---

## 4. Issue 3 — The stranded v0.3.0 branch

The relicense + IA-conformance + tech-debt work is on
`feature/relicense-ia-conformance-0.3.0`, **not merged** to `develop` or `main`.

- `develop` (this branch's base) is still **MIT OR Apache-2.0**, pre-conformance, and
  *still contains the retrieval files* (the deletion is only on the v0.3.0 branch).
- The v0.3.0 branch has: AGPL-3.0 + LICENSE-COMMERCIAL, SPDX headers, `CONTRIBUTING.md`,
  `rust-toolchain.toml`, `HANDOFF.md`, `docs/ROADMAP.md`, the new `holographic_ops` bench,
  `tests/integration/end_to_end.rs`, and the retrieval deletions.

**Why it matters for Kagome:** Kagome is AGPL-3.0-only. For the integration to be
licensing-coherent, Minuet needs to be on the AGPL track too. Merging the v0.3.0 branch
(even if the retrieval deletion is then partially reverted per §2) is a prerequisite.

### Decision: base branch sequencing (⚠ pick first)

| Option | Sequence |
|--------|----------|
| **A (recommended)** | Merge `feature/relicense-ia-conformance-0.3.0` → `develop` first (re-establish AGPL/conformance baseline), then branch the readiness sprint off that, reverting/re-expressing the retrieval deletions per §2. |
| **B** | Branch the readiness sprint off current `develop` (still MIT/Apache, has the retrieval files), carry the relicense forward together. Risks conflicting with the stranded branch. |

**Recommendation: A.** Land the relicense as-is first; it's been finished and stranded.
Then this sprint starts from a clean AGPL/conformant base and the only contested change
(the retrieval deletion) is a localized revert/re-express, not a license entanglement.

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
so the bind/bundle logic written here is reused there. Recommend implementing behind the
existing `optical` feature with a software (`MockOpticalHardware`) reference, so it's
testable without physical hardware.

---

## 6. Other readiness points

- **CI.** Verify a CI workflow exists and runs `fmt`, `clippy -D warnings`, the test
  matrix (default + `optical` + `full`), and `cargo doc`. The v0.3.0 branch added
  `rust-toolchain.toml` (nightly) — confirm CI uses it. IA standard: see
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

Ordered by dependency; each item is a PR against `develop` (after §4 sequencing decision).

| # | Workstream | Depends on | Approx scope |
|---|-----------|------------|--------------|
| 0 | **Merge v0.3.0 relicense branch → develop** (§4-A) | — | merge + resolve; re-establish AGPL/conformance baseline |
| 1 | **Bump `amari-holographic` floor `^0.15` → `^0.23`** (§3-A) | 0 | Cargo.toml + adapt to API drift; 69-test safety net |
| 2 | **Restore `temperature.rs`** (§2-B) | 0 | near-verbatim; pure Minuet, no new dep |
| 3 | **Re-express `Attribution` over `BindingAlgebra`** (§2-B) | 1 | port `attribution.rs` from `TropicalDualClifford` to generic `A` |
| 4 | **Wire annealed temperature into `ResonatorRetriever`** (§2-B) | 2,3 | close the loop: annealed cleanup as a retriever option |
| 5 | **Implement `optical_store` compute path** (§5) | 1 | bind+bundle via `OpticalFieldAlgebra`; Mock-hardware reference impl; coordinate with Kagome |
| 6 | **CI + doc hygiene pass** (§6) | 0 | fmt/clippy/test/doc matrix; un-ignore doc-tests where possible |

Items 1–4 are the "amari seam + fusion restore" core. Item 5 is the Kagome-shared compute
path. Items 0 and 6 are housekeeping that should bookend the sprint.

---

## 9. Verification checklist (before declaring the sprint done)

- [ ] `develop` is AGPL-3.0-only with SPDX headers on all `.rs` files (§4).
- [ ] `amari-holographic` resolves to 0.23.x in the lockfile (§3).
- [ ] `cargo test --all-features` green; test count ≥ 69 and *increased* by restored
      retrieval coverage (§2).
- [ ] `Attribution` and `TemperatureSchedule` are reachable from the public API and have
      doc-tests (§2).
- [ ] `optical_store` performs a real bind+bundle verifiable via `MockOpticalHardware`
      (§5).
- [ ] `cargo clippy --all-features -- -D warnings` and `cargo doc --no-deps` clean.
- [ ] Kagome's `--features minuet` build still passes against the updated Minuet (§7).

---

## 10. Open decisions (consolidated)

1. **Base sequencing (§4):** merge v0.3.0 relicense branch to `develop` first? *(rec: yes)*
2. **amari seam (§3):** bump floor to `^0.23`? *(rec: yes, as first PR)*
3. **Fusion restore strategy (§2):** verbatim / generic re-express / defer? *(rec: generic
   re-express — option B)*
4. **`optical_store` (§5):** implement now behind `optical` feature with mock hardware?
   *(rec: yes, coordinate with Kagome)*

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
- v0.3.0 handoff (predecessor): `HANDOFF.md` on `feature/relicense-ia-conformance-0.3.0`.
