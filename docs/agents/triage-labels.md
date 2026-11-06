# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker. The labels are team labels in Linear team ROZ.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

## Labels only

Triage and grilling change labels only; the issue's state is the user's call. Apply a label with an `issueUpdate` that touches labels alone:

```graphql
issueUpdate(id: "<issue-uuid>", input: {
  addedLabelIds: ["<new-label-id>"],
  removedLabelIds: ["<previous-triage-label-id>"]
})
```

An issue carries at most one triage label: remove the previous one when applying the next. Look up ids as described in `docs/agents/issue-tracker.md`.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
