# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker. The labels are team labels in Linear team ROZ.

| Label in mattpocock/skills | Label in our tracker | Linear state  | Meaning                                  |
| -------------------------- | -------------------- | ------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | `Backlog`     | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | `Needs Info`  | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | `Planned`     | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | `Planned`     | Requires human implementation            |
| `wontfix`                  | `wontfix`            | `Canceled`    | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

## Keep label and state in sync

Applying a triage label always moves the issue to the state in the same row, in a single `issueUpdate`:

```graphql
issueUpdate(id: "<issue-uuid>", input: {
  addedLabelIds: ["<new-label-id>"],
  removedLabelIds: ["<previous-triage-label-id>"],
  stateId: "<state-id>"
})
```

An issue carries at most one triage label: remove the previous one when applying the next. Look up ids as described in `docs/agents/issue-tracker.md`.

Edit the "Label in our tracker" and "Linear state" columns to match whatever vocabulary you actually use.
