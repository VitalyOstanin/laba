# 13. Backend abstraction: trait, capabilities, and typed cross-backend entities

Date: 2026-07-18

## Status

Accepted

## Context

laba's goal is to help the user not miss tasks, pull requests, and notifications
across several trackers, extensibly. The near-term backend horizon is GitHub,
Jira, YouTrack, and OpenProject (OpenProject stays as an implementation but is no
longer the reference shape).

The original model does not scale to that horizon:

- `Backend` is a closed enum with ~12 `matches!(self, …)` capability methods, and
  every core operation dispatches with its own `match`. Adding a backend edits all
  of them — there is no single extension point.
- Tasks and notifications are untyped JSON (`serde_json::Value` in Rust,
  `Record<string, unknown>` in the frontend), "normalized to the same shape" by
  convention. The shape is unenforced, so field drift between backends is silent
  (e.g. GitHub notifications lacked a `read` flag until it was noticed by a bug).
- Capabilities are a growing bag of booleans that cannot express nuance (GitHub's
  read toggle is one-way, OpenProject's is two-way — squeezed into one bool with a
  comment).
- Task identity differs per backend (`owner/repo#7`, `PROJ-45`, `2-45`, `#123`) but
  is a stringly-typed `id`.
- There is no field for *why* an item is in the user's list, so "a PR awaiting my
  review" is indistinguishable from "a task assigned to me" — which the product
  goal fundamentally needs to tell apart.

Config backward compatibility is explicitly not required, so the schema may change
freely.

## Decision

Introduce a backend abstraction in three parts:

1. **`trait Backend`** replaces the enum as the carrier of behavior. Each backend
   is a module implementing the trait; a factory builds a `Box<dyn Backend>` from a
   `ServerProfile`. `BackendKind` remains as a serde/identity tag only. Core
   operations call the trait, not a per-backend `match`.

2. **`Capabilities` struct** returned per backend, with enums for nuance
   (`ReadToggle::{None,OneWay,TwoWay}`, `DetailSupport::{None,InApp}`,
   `TimelogSupport::{None,Basic,WithActivities}`) instead of loose booleans.
   `ServerInfo` embeds it as a nested object; the frontend reads `capabilities.*`.

3. **Typed cross-backend entities** replace `Value`: `Task`, `Notification`,
   `TaskDetail`, `Comment`, `TimeEntry`. `Task` carries `kind`
   (Issue/PullRequest/WorkPackage/Other) and `reason`
   (Assigned/Authored/ReviewRequested/Mentioned/Involved/Own) — the two axes the
   product goal needs — plus a normalized `status_category` alongside the raw
   `status`. Task identity is `TaskId { display, raw }` (a display string plus the
   raw id for API calls), not a per-backend enum: simpler, and each backend parses
   its own `raw`. Entities serialize camelCase so the frontend gets real
   interfaces.

Rollout is phased: (1) trait + capabilities, (2) typed entities, (3) Jira/YouTrack
implementations. All four backends are designed up front (see the design spec); GitHub
and OpenProject are implemented concretely, while Jira and YouTrack are designed but not
yet implemented.

## Consequences

- Adding a backend becomes a local change (one module implementing the trait plus
  its field mapping), not edits across every capability method and dispatch.
- Field drift between backends becomes a Rust compile error instead of a silent
  runtime bug.
- The product goal is expressible: `kind` + `reason` let the UI rank and filter a
  review-requested PR distinctly from an assigned task.
- Capability nuance (one-way vs two-way read, timelog with/without activities) is
  typed, so the UI branches on a variant rather than a bool-plus-comment.
- `TaskId { display, raw }` gives no compile-time guarantee that `raw` parses; that
  correctness stays the owning backend's responsibility. Accepted for simplicity
  over a per-backend enum whose variants every call would destructure anyway.
- The frontend contract changes (typed entities, nested capabilities); the config
  schema changes too, which is acceptable since backward compatibility is not
  required.
- Concepts are not universal: pull requests are a GitHub-specific `TaskKind`, and a
  notification inbox may be absent (Jira) — handled by `capabilities.notifications`
  hiding the column rather than by special-casing.
