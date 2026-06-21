# Retrofit Audits

Use this file when an existing adapter component predates the current adapter
workflow, the latest gold-standard component, or a new documented convention.
Retrofit audits are implementation tasks, not report-only reviews.

## Issue And Evidence

Create a fresh issue for the audit instead of reopening older completed
implementation issues. The new issue is the execution record and the PR should
close it. Reference older issues only as prior context.

Treat old browser screenshots, E2E results, and widget smoke evidence as stale
until each supported outcome has been remapped to the current final outcome
matrix. A prior passing check proves only the older contract.

## Gold-Standard Comparison

Before editing code, choose the current gold-standard adapter component for the
same class of problem. Checkbox is the default for low-level form primitives
until another component explicitly supersedes it.

Compare the audited component against that component for:

- root naming and compound part naming;
- prelude export shape;
- public styling surface for every exposed part and structural node;
- fallback structural-node behavior;
- shared helper extraction versus duplicated adapter logic;
- adapter SSR/unit tests, wasm tests, E2E harness axes, widgets coverage, and
  browser evidence;
- docs and specs that describe the current contract.

Do not copy API names blindly. Copy conventions only when they serve the same
contract. Record intentional differences in the sketch matrix.

## Public Primitive Renames

When a retrofit renames, removes, or privatizes a low-level primitive or prop
type, run a stale-symbol scan before handoff. Search at least:

- adapter source and preludes;
- adapter tests and wasm tests;
- E2E fixtures and harnesses;
- all widgets crates;
- adapter specs and foundation docs;
- lint heuristics and source snippets under `xtask`;
- implementation sketches and usage notes.

For root primitive renames, scan for old exported functions, old `Props` names,
old public helper functions, planned-but-private part names,
`module::OldName` call sites, deep imports, and spec snippets. Update all
consumer-facing examples in the same PR unless the old name remains as a
documented compatibility alias.

## Closed Adapter API Split

When a retrofit moves an older closed-anatomy adapter component to the current
primitive workflow, treat it as a breaking adapter API audit unless an explicit
compatibility alias is approved. The target shape is:

- remove the public closed component from `ars-leptos` / `ars-dioxus` or hide
  any temporary compatibility alias from the prelude;
- expose `Root` plus the public unstyled parts that consumers can safely
  compose without rebuilding component-owned behavior;
- keep behavior-critical subparts private when standalone exposure would make
  consumers own focus, keyboard, ARIA, drag/drop, close, or localized-message
  policy; document those exceptions and their supported customization path;
- keep one typed collection source for collection-backed components, and use
  typed renderers when consumers need row anatomy customization without
  repeating keys;
- move the ready-made visual component into `ars-leptos-components` and
  `ars-dioxus-components` as category-first CSS and Tailwind source
  templates;
- update plain widgets to demonstrate direct primitive composition where
  useful, and update CSS/Tailwind widgets to consume the styled source
  templates;
- update E2E fixtures to exercise the adapter primitives directly, with
  widget/browser smoke covering the styled high-level component.

The retrofit sketch must explicitly say which old monolithic conclusions are
superseded and which public names were removed, replaced, or intentionally
kept as compatibility aliases.

## Retrofit Stop Condition

A retrofit audit is complete only when the updated sketch records:

- the chosen gold-standard component and why it applies;
- reference evidence refreshed for the current task;
- local evidence refreshed after the final code shape;
- final matrix rows with no `Unknown`, `Unverified`, `ContractGap`,
  `AdapterApiGap`, or `WidgetOnlyWorkaround` status;
- every intentional difference and out-of-scope axis.

If the audit reveals a reusable rule, update this workflow, the adapter
checklists, or the relevant framework skill in the same PR.
