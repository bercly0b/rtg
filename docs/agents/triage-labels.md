# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker. The labels are team labels in Linear team ROZ.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-grilling`     | Needs grilling before it can be built    |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

There is no `needs-triage` label in this tracker. Triage answers whether an incoming issue gets done at all, and every rtg issue is filed by the maintainer, so that is already settled. What a fresh issue still lacks is grilling, hence `needs-grilling`.

## Labelling a new issue

`needs-grilling` is not the default for a new issue. When creating one, judge what it still lacks:

- nothing: the description settles what to build and how to verify it, and an agent could implement it without asking anything → `ready-for-agent` (or `ready-for-human` if a human has to do it);
- open decisions remain (behaviour, scope, design) → `needs-grilling`.

## Labels only

Triage and grilling change labels only; the issue's state is the user's call. Apply a label with an issue update that touches labels alone: `addLabels` with the new label and `removeLabels` with the previous triage label, both by name (see `docs/agents/issue-tracker.md`).

An issue carries at most one triage label: remove the previous one when applying the next.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
