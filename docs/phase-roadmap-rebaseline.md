# RTG phase roadmap rebaseline (2026-02)

This document is the source of truth for RTG phase numbering **after** the rebaseline.

## Why this exists

Phase numbering was shifted. Starting now, **Phase 3 means real Telegram auth/backend integration**.

## Legacy-to-current phase mapping

| Legacy reference | Current reference | Notes |
| --- | --- | --- |
| Phase 1 | Phase 1 | Unchanged: layer contracts and data-flow baseline. |
| Phase 2 | Phase 2 | Unchanged: quality gate execution contract. |
| Legacy Phase 3 | Phase 4 | Renumbered because a new Phase 3 was inserted. |
| _N/A in legacy plan_ | **Phase 3 (new)** | Real Telegram auth/backend implementation. |

## Interpretation rule (mandatory)

When older notes/issues/PR comments mention "Phase 3", treat it as:

- **Phase 4** if the context is the old plan.
- **Phase 3** if the context explicitly targets real Telegram auth/backend work under the new plan.

When ambiguity exists, prefer explicit wording: `legacy Phase 3` or `new Phase 3`.

## Related docs

- [`docs/phase1-layer-data-flow.md`](phase1-layer-data-flow.md)
- [`docs/phase2-quality-gate.md`](phase2-quality-gate.md)
- [`docs/auth-connectivity-status-model.md`](auth-connectivity-status-model.md)
