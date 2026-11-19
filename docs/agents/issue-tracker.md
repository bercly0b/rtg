# Issue tracker: Kaneo

Issues and specs for this repo live in a self-hosted Kaneo: project **rtg** (key `RTG`). The workspace is shared with other projects, so scope every read and write to the `rtg` project.

- Workspace id: `FpWEAKYkkAovBm31yoBZUVHUQ5BcaMtE`
- Project id: `czkwiouckexygdgf6nhf649p`

Kaneo calls an issue a **task**; the two words mean the same thing here.

## Access

The Kaneo MCP server (`kaneo`, tools named `mcp__kaneo__*`). If its tools are missing, the server is not connected or not authorised: say so and ask the user to run `/mcp`.

The MCP server cannot link a resource to a task. That one operation goes through the Kaneo REST API (see **Link a pull request**), which needs two environment variables: `KANEO_API_URL` (the instance's root URL, without `/api`) and `KANEO_API_KEY`. If either is unset, say so and ask the user; never print the key. Use the REST API for nothing else.

Ticket identifiers look like `RTG-118`: fetch a task by one with `get_task_by_ticket_id`. Every other tool takes the task's id, which that call returns. Tools take ids, not names: use the ids above, a column slug for a status, and the `id` from `whoami` for the current user.

`list_tasks` is paginated: fetch every page up to `pagination.totalPages` before treating the list as complete. `search` needs the workspace id.

### Issues migrated from Linear

The history was moved from Linear (team ROZ) and renumbered: `ROZ-239` is now `RTG-118`. Git history, changelog fragments and older docs still say `ROZ-<n>`. Each migrated task starts its description with `Migrated from Linear ROZ-<n>`, so find it with `search` for `ROZ-<n>` and take the task whose first line matches. New work uses `RTG-<n>` only.

## Conventions

- **Language**: write issue titles and descriptions in Russian, matching the existing rtg issues.
- **Create an issue**: `create_task` in project rtg with a title, a markdown description, status `to-do` (the `Backlog` column) and priority `no-priority`. New issues stay in `Backlog`: only the user moves issues to `Planned`. Choose the label per **Labelling a new issue** in `docs/agents/triage-labels.md`: a clear issue is `ready-for-agent` straight away, not `needs-grilling`.
- **Read an issue**: fetch it by ticket identifier for its status and assignee. Labels, relations, comments and linked resources are not part of the task: read labels as in **Apply / remove labels**, relations with `get_task_relations`, comments with `list_task_comments`.
- **List issues**: `list_tasks` for project rtg, optionally filtered by status. Each task comes with its labels and `externalLinks`. For open only, keep the tasks in columns that are not final: everything except `done` and `canceled`.
- **Make an issue a sub-issue of a parent**: `create_task_relation` with type `subtask`, the parent as source and the child as target.
- **Comment on an issue**: `create_task_comment` with a markdown body.
- **Apply / remove labels**: a label on a task is its own copy of a workspace label. `list_workspace_labels` returns both kinds: workspace labels have `taskId: null`, copies carry the task's id. To read a task's labels, keep the copies with its id. To apply a label, `create_label` with the workspace id, the task id and the name and colour of the workspace label. To remove one, `delete_label` with the id of the task's copy. Never delete a label with `taskId: null`: that removes it from every task. For triage labels, see `docs/agents/triage-labels.md`.
- **Change status**: `update_task_status` with the column slug.

  | Column      | Slug          |
  | ----------- | ------------- |
  | Backlog     | `to-do`       |
  | Planned     | `plann3d`     |
  | In Progress | `in-progress` |
  | In Review   | `in-review`   |
  | Need info   | `need-info`   |
  | Done        | `done`        |
  | Canceled    | `canceled`    |

- **Close**: comment with the reason, then move to `done` (or `canceled` for won't-fix and duplicates).

## Link a pull request

Every pull request made for a ticket is linked to that ticket as a resource, right after `gh pr create`. A comment with the URL does not count. A task can carry several pull requests: link each one.

```sh
curl -sS -X POST "$KANEO_API_URL/api/external-link/task/<task id>" \
  -H "Authorization: Bearer $KANEO_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"url": "<pull request URL>", "title": "<pull request title>", "resourceType": "pull_request", "metadata": {"state": "open"}}'
```

`<task id>` is the task's id, not the ticket identifier. Verify with `GET` on the same path, or by the task's `externalLinks` in `list_tasks`.

## When a skill says "publish to the issue tracker"

Create a Kaneo task in project rtg.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single rtg issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: a sub-issue of the map (see **Make an issue a sub-issue of a parent**). Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev. A `wayfinder:*` label that does not exist yet is created by applying it (see **Apply / remove labels**); pick any colour and reuse it for later copies.
- **Blocking**: Kaneo's task relations. Mark the blocker as blocking the blocked ticket with `create_task_relation`, type `blocks`, the blocker as source. A ticket's blockers are the `blocks` relations from `get_task_relations` where it is the target: read each blocker for its status. A ticket is unblocked when every blocker is in `done` or `canceled`.
- **Frontier query**: list the map's sub-issues (its `subtask` relations), keep the open ones, drop any with an open blocker or an assignee; first in map order wins.
- **Claim**: assign the ticket to the current user (`update_task_assignee` with the `id` from `whoami`) and move it to `in-progress`, the session's first write.
- **Resolve**: comment with the answer, move the ticket to `done`, then append a context pointer (gist + link) to the map's Decisions-so-far.
