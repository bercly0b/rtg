# Issue tracker: Linear

Issues and specs for this repo live in Linear: team **ROZ** (Rozov-saas), project **rtg**. The team is shared with other projects, so scope every read and write to the `rtg` project.

- Team id: `ebfa7747-6e67-416e-ba4b-e2df75090c72`
- Project id: `643d1611-f48e-4543-ba99-00928d349223`
- Project URL: https://linear.app/rozov-saas/project/rtg-f1b529b0919a

## Access

GraphQL API, single endpoint: `POST https://api.linear.app/graphql`.

The token is `$LINEAR_API_KEY`, defined in `~/.zshrc`, so it is absent from non-interactive shells. Load it with `export LINEAR_API_KEY=$(zsh -ic 'printf %s "$LINEAR_API_KEY"')`. Send it raw in the `Authorization` header, no `Bearer`. Never print the token.

Errors come back as HTTP 200 with an `errors` array: check it before reporting success.

Build request bodies with `jq` and GraphQL variables instead of escaping markdown into JSON by hand:

```bash
jq -n \
  --arg q 'mutation($input: IssueCreateInput!) { issueCreate(input: $input) { success issue { identifier url } } }' \
  --arg title "..." --arg description "..." \
  '{query: $q, variables: {input: {
      teamId: "ebfa7747-6e67-416e-ba4b-e2df75090c72",
      projectId: "643d1611-f48e-4543-ba99-00928d349223",
      title: $title, description: $description}}}' \
| curl -s https://api.linear.app/graphql -H "Authorization: $LINEAR_API_KEY" \
    -H 'Content-Type: application/json' --data-binary @- | jq
```

Issue identifiers look like `ROZ-182`. `issue(id: ...)` accepts either the identifier or the UUID; for mutations (`issueUpdate(id: ...)`, `parentId`, `issueId`, `relatedIssueId`) use the UUID from `issue(id: "ROZ-182") { id }`.

## Conventions

- **Language**: write issue titles and descriptions in Russian, matching the existing rtg issues.
- **Create an issue**: `issueCreate(input: {teamId, projectId, title, description})`. `description` is markdown. New issues land in `Backlog` and stay there: never pass `stateId` when creating an issue. Only the user moves issues to `Planned`.
- **Read an issue**: `issue(id: "ROZ-42") { id identifier title description url state { name } labels { nodes { name } } parent { identifier } children { nodes { identifier title state { name } } } comments { nodes { body user { name } } } }`.
- **List issues**: `project(id: "643d1611-f48e-4543-ba99-00928d349223") { issues(first: 50, filter: {...}) { nodes { identifier title state { name } labels { nodes { name } } } } }`. Open only: `state: { type: { nin: ["completed", "canceled", "duplicate"] } }`. By label: `labels: { some: { name: { eq: "ready-for-agent" } } }`.
- **Make an issue a sub-issue of a parent**: pass the parent's UUID as `parentId` in `issueCreate`, or `issueUpdate(id: "<child>", input: {parentId})` afterwards.
- **Comment on an issue**: `commentCreate(input: {issueId, body})`.
- **Apply / remove labels**: `issueUpdate(id, input: {addedLabelIds: [...], removedLabelIds: [...]})`. Look label ids up with `issueLabels(filter: {name: {in: ["needs-triage", "ready-for-agent"]}}) { nodes { id name } }`. For triage labels, see `docs/agents/triage-labels.md`.
- **Change state**: `issueUpdate(id, input: {stateId})`. States are per-team; list them with `team(id: "ebfa7747-6e67-416e-ba4b-e2df75090c72") { states { nodes { id name } } }`. ROZ states: `Backlog`, `Planned`, `In Progress`, `Needs Info`, `In Review`, `Done`, `Canceled`, `Duplicate`.
- **Close**: comment with the reason, then move to `Done` (or `Canceled` for won't-fix).

## When a skill says "publish to the issue tracker"

Create a Linear issue in team ROZ, project rtg.

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single rtg issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: a sub-issue of the map (see **Make an issue a sub-issue of a parent**). Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev. Create missing `wayfinder:*` labels in team ROZ on first use: `issueLabelCreate(input: {name, teamId})`.
- **Blocking**: Linear's native issue relations. Add an edge with `issueRelationCreate(input: {type: blocks, issueId: "<blocker-uuid>", relatedIssueId: "<blocked-uuid>"})`. A ticket's blockers are its `inverseRelations { nodes { type issue { identifier state { type } } } }` with `type == "blocks"`. A ticket is unblocked when every blocker's state type is `completed` or `canceled`.
- **Frontier query**: list the map's open `children`, drop any with an open blocker or an assignee; first in map order wins.
- **Claim**: `issueUpdate(id, input: {assigneeId: "<viewer-id>", stateId: "<In Progress>"})` with the id from `viewer { id }`, the session's first write.
- **Resolve**: `commentCreate` with the answer, move the ticket to `Done`, then append a context pointer (gist + link) to the map's Decisions-so-far.
