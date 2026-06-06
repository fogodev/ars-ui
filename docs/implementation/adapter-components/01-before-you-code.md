# Before You Code

Adapter component work starts from the assigned task and the component specs,
not from an example page or a convenience implementation.

## Implementation Discipline

Do not take shortcuts. When the issue, spec, plan, parity review, or review
finding calls for a specific implementation shape, implement that shape unless
you first prove it is technically impossible or incorrect.

In particular:

- do not replace renderer-independent adapter behavior with a browser-only or
  target-only workaround unless the spec says the behavior is target-specific;
- do not duplicate shared behavior separately in Leptos and Dioxus. Selection
  semantics, disabled behavior, section traversal, layout metadata,
  drag/drop validity and reorder math, load-more suppression, hover/press/drop
  state, and reference-parity rules belong in `crates/ars-components` or
  another shared crate first;
- do not ship a partial adapter and rely on follow-up work to restore the full
  contract;
- do not keep known semantic differences between Leptos and Dioxus merely
  because only one adapter exposes the currently reported reproduction;
- do not use "works in the example" as evidence that the component contract is
  satisfied;
- if the clean implementation has a real blocker, document that blocker with
  code evidence before choosing an alternative.

## Dependency Blockers

Before moving an adapter task to In Progress, check dependency metadata:

```bash
cargo xtask spec component-deps <component> --adapter leptos
cargo xtask spec component-deps <component> --adapter dioxus
cargo xtask spec issue-deps --adapter leptos --component <component> --dry-run
cargo xtask spec issue-deps --adapter dioxus --component <component> --dry-run
```

Hard blockers come from `spec/manifest.toml` `component_deps` entries with
`kind = "requires"` or `kind = "composes", blocking = true`, plus the adapter
foundation dependencies shown by the issue-dependency report. Those blockers
must exist in both the issue body `Depends on` section and GitHub's native
`blocked_by` graph before the task is pickable.

`kind = "boundary"` entries are not blockers. They document feature ownership
limits, such as "grid layout belongs to GridList, not Listbox", and should
appear as issue notes instead of native dependencies. See
[adapter-component-dependencies.md](../adapter-component-dependencies.md).

## Required Reading

Before editing code, read:

- the assigned GitHub issue and acceptance criteria;
- the framework-agnostic component spec under
  `spec/components/<category>/`;
- the matching adapter specs under `spec/leptos-components/<category>/` and
  `spec/dioxus-components/<category>/` when both adapters are in scope;
- the relevant adapter foundation specs:
  `spec/foundation/08-adapter-leptos.md` and
  `spec/foundation/09-adapter-dioxus.md`;
- [adapter-contract.md](../adapter-contract.md);
- [widgets-ownership.md](../../../examples/widgets-ownership.md).

When touching Leptos or Dioxus code or specs, load the repo skill for that
framework before relying on framework APIs.

## Reference Parity Baseline

Before planning the adapter API or examples, inspect the live documentation page
for the strongest mature counterpart in this order:

1. React Aria / React Spectrum;
2. Ark UI / Chakra UI;
3. Radix UI / shadcn/ui;
4. another mature component library only when the first three do not cover the
   primitive or feature axis.

The implementation target is maximum practical outcome parity with that
reference. Do not claim full parity unless the component spec contains a real
feature matrix showing every supported, unsupported, and not-applicable axis
with reasons.

Before coding, create an implementation sketch under
`docs/implementation/sketches/` following
[10-reference-exploration-sketch.md](10-reference-exploration-sketch.md). The
sketch must be based on live `playwright-cli` interaction with the reference
page, not only static docs. Treat the sketch as the implementation plan's source
of truth; update it when the work discovers new axes or contract gaps.

## Planning Output

Every adapter implementation plan must include:

- blocker-check command results;
- spec files read;
- reference-exploration sketch path;
- counterpart outcome matrix source URLs, artifact paths, and feature axes;
- shared agnostic work required before adapter wiring;
- adapter crate deliverables;
- adapter test deliverables;
- E2E fixture, harness, matrix, axe, and visual deliverables;
- widgets deliverables for all six widgets crates;
- validation commands.
