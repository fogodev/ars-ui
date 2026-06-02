# Adapter Component Dependencies

Adapter component work uses `spec/manifest.toml` as the source of truth for
component-to-component dependencies and feature boundaries. GitHub issue bodies
and native issue dependencies are synchronized from that metadata.

## Dependency Kinds

- `requires`: hard blocker. The dependent adapter task must not start until the
  required component task is complete.
- `composes`: the component renders or delegates to another adapter component.
  It blocks issue pickup only when `blocking = true`.
- `boundary`: explicit non-dependency. Use this when a counterpart library has a
  feature that belongs to another ars-ui component, such as React Aria
  `layout: "grid"` belonging to GridList rather than Listbox.
- `related`: non-blocking conceptual relationship.

Only `requires` and blocking `composes` entries become native GitHub
`blocked_by` dependencies. `boundary` entries must be written into issue bodies
as non-blocking notes, never as blockers.

## Required Checks Before Picking Adapter Work

Before starting an adapter task, run:

```bash
cargo xtask spec component-deps <component> --adapter leptos
cargo xtask spec component-deps <component> --adapter dioxus
```

For a whole category or triage pass, run:

```bash
cargo xtask spec component-deps --all --adapter leptos
cargo xtask spec component-deps --all --adapter dioxus
```

Then verify GitHub issue dependencies:

```bash
cargo xtask spec issue-deps --adapter leptos --component <component> --dry-run
cargo xtask spec issue-deps --adapter dioxus --component <component> --dry-run
```

If the dry-run reports missing native dependencies, sync them before moving the
task to In Progress. If it reports a `boundary` note, keep the task pickable but
carry that note into the issue body and PR scope.

## Maintenance Rule

When implementation reveals a new component dependency or feature boundary,
update all of these in the same PR or sync pass:

1. `spec/manifest.toml` `component_deps`;
2. the relevant component or adapter spec text when the relationship affects
   the public contract;
3. the GitHub task issue `Depends on` / `Component dependency notes` sections;
4. native GitHub issue dependencies for hard blockers.

Do not infer blockers from visual similarity alone. A relationship is blocking
only when the manifest marks it as `requires` or as `composes` with
`blocking = true`.
