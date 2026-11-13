# Issue tracker: Linear

Issues and specs for this repo live in Linear: team **ROZ** (Rozov-saas), project **rtg**. The team is shared with other projects, so scope every read and write to the `rtg` project.

- Team id: `ebfa7747-6e67-416e-ba4b-e2df75090c72`
- Project id: `643d1611-f48e-4543-ba99-00928d349223`
- Project URL: https://linear.app/rozov-saas/project/rtg-f1b529b0919a

## Access

The Linear MCP server (`linear`, tools named `mcp__linear__*`), authorised over OAuth: no API key is involved. If its tools are missing, the server is not connected or not authorised: say so and ask the user to run `/mcp`, do not fall back to the GraphQL API.

Issue identifiers look like `ROZ-182`; the tools accept either the identifier or the UUID. Pass names where a tool takes them (team `ROZ`, project `rtg`, a state or label name, `me` for the current user) and the ids above where it wants an id.

Lists are paginated: when a result has `hasNextPage`, fetch the next page with its cursor before treating the list as complete.

## Conventions

- **Language**: write issue titles and descriptions in Russian, matching the existing rtg issues.
- **Create an issue**: team ROZ, project rtg, title and a markdown description. New issues land in `Backlog` and stay there: never pass a state when creating an issue. Only the user moves issues to `Planned`.
- **Read an issue**: fetch it by identifier for its state, labels and parent. Relations come back only when asked for (`includeRelations`). Sub-issues and comments are not part of the issue: list issues by parent, and list the issue's comments.
- **List issues**: always filter by project `rtg`. Narrow by label, e.g. `ready-for-agent`, or by state. The state filter takes one state name or state type and cannot negate, so for open only list without it and keep the issues whose `statusType` is `backlog`, `unstarted` or `started`.
- **Make an issue a sub-issue of a parent**: set the parent when creating the issue, or update the child's parent afterwards.
- **Comment on an issue**: create a comment on it with a markdown body.
- **Apply / remove labels**: update the issue with `addLabels` / `removeLabels`, by label name. Never use `labels`: it replaces the whole label set. For triage labels, see `docs/agents/triage-labels.md`.
- **Change state**: update the issue's state by name. ROZ states: `Backlog`, `Planned`, `In Progress`, `Needs Info`, `In Review`, `Done`, `Canceled`, `Duplicate`.
- **Close**: comment with the reason, then move to `Done` (or `Canceled` for won't-fix).

## When a skill says "publish to the issue tracker"

Create a Linear issue in team ROZ, project rtg.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single rtg issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: a sub-issue of the map (see **Make an issue a sub-issue of a parent**). Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev. Create missing `wayfinder:*` labels on first use, passing the team id above: without it the label is created for the whole workspace.
- **Blocking**: Linear's native issue relations. Mark the blocker as blocking the blocked ticket (`blocks` / `blockedBy`) when creating or updating either issue. A ticket's blockers are its `blockedBy` relations, which carry only identifier and title: read each blocker for its state. A ticket is unblocked when every blocker is in `Done`, `Canceled` or `Duplicate`.
- **Frontier query**: list the map's sub-issues (issues whose parent is the map), keep the open ones, drop any with an open blocker or an assignee; first in map order wins.
- **Claim**: assign the ticket to the current user (`me`) and move it to `In Progress`, the session's first write.
- **Resolve**: comment with the answer, move the ticket to `Done`, then append a context pointer (gist + link) to the map's Decisions-so-far.
