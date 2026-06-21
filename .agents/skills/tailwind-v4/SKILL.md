---
name: tailwind-v4
description: Tailwind CSS v4 guidance for writing or reviewing Tailwind classes in ars-ui source templates, widgets, and docs. Use when editing Tailwind class strings, Tailwind widget examples, ars-*-components tailwind variants, tailwind.css files, or when VS Code/Tailwind language server reports class canonicalization warnings.
---

# Tailwind v4

Use this skill before editing Tailwind class strings in Rust `view!` / `rsx!`
markup, styled source templates, widgets, or Tailwind CSS files.

## Current Docs Check

When syntax is uncertain, check the official Tailwind docs first. Prefer these
pages:

- `https://tailwindcss.com/docs/hover-focus-and-other-states`
- `https://tailwindcss.com/docs/detecting-classes-in-source-files`
- `https://tailwindcss.com/docs/functions-and-directives`

Tailwind changes class syntax and editor diagnostics between releases; do not
guess when an editor warning, new variant, or v4 migration detail is involved.

## Class Authoring Rules

- Prefer built-in variants over arbitrary variants. Use `hover:`, `focus:`,
  `focus-visible:`, `disabled:`, `aria-disabled:`, `data-ars-selected:`, and
  `group-data-ars-selected:` before reaching for bracketed selectors.
- For presence-only `data-*` attributes, use the v4 named data variant:
  `data-ars-selected:bg-blue-600`, not the bracketed arbitrary-selector form.
  For parent group state, use `group-data-ars-selected:bg-white/20`, not
  the bracketed group-data form.
- Use bracketed data variants only when the selector needs a value or other
  syntax that the named variant cannot express, such as
  `data-[size=large]:p-8`.
- Use arbitrary variants for real custom selectors, for example
  `[&:not([data-ars-disabled]):not([data-ars-selected]):hover]:bg-white`.
  If the selector is repeated or is only compensating for missing component
  state, add a renderer-independent `data-ars-*` attr on the public part
  instead.
- Avoid `:has()`, `has-*`, and `group-has-*` for component-owned state when a
  public part can receive mirrored agnostic state. `:has()` is valid Tailwind,
  but it is a fallback for descendant-dependent styling, not the default
  contract for source templates.
- Keep Tailwind source-template classes inline in the copied component markup.
  Do not hide normal class lists in constants or external component-specific
  CSS.
- Do not build class names dynamically. Tailwind scans source as plain text, so
  every utility class must exist as a complete static token.
- Use `@apply` only in CSS variants or true custom CSS integration points.
  Tailwind variants in `ars-*-components` should be styleable by editing Rust
  class strings alone.
- Treat cursor utilities as part of inline component state, not external CSS.
  Use `cursor-pointer` on clickable shells and child controls,
  `data-ars-disabled:cursor-not-allowed` or
  `aria-disabled:cursor-not-allowed` for disabled states,
  `data-ars-dragging:cursor-grabbing` on the public drag source, and
  `group-data-ars-dragging:cursor-grabbing` on child parts whose own cursor
  utilities would otherwise override the shell while dragging.

## ars-ui Source-Template Boundary

Tailwind variants in `ars-leptos-components`, `ars-dioxus-components`, and the
Tailwind widgets must be complete without component-specific rules in
`tailwind.css`. If a supported visual state cannot be reached with inline
classes on public parts, fix the component surface or agnostic attrs first.

Tailwind widget galleries must use the high-level Tailwind component from
`ars-*-components`. Do not manually compose low-level adapter parts in
`examples/widgets-*-tailwind`; that bypasses the source template being
demonstrated and duplicates the class contract. Keep direct primitive
composition in the unstyled widget crates, adapter tests, and E2E fixtures.

CSS variants may use adjacent `.css` files. Tailwind variants should keep state
styling in classes on `Root`, `List`, `TabShell`, `Trigger`, `CloseTrigger`,
`Panel`, or the equivalent public parts.

For compound draggable parts, make the drag cursor state visible on both the
state-owning shell and its interactive descendants. A close affordance, icon
button, or label inside a dragged shell should not keep `cursor-pointer` while
the shell is in `data-ars-dragging`.

## Warning Sweep

Before finishing a Tailwind edit, run scans over touched Tailwind surfaces:

```bash
rg -n 'data-\[ars-[a-z-]+\]|group-data-\[ars-[a-z-]+\]' <paths>
rg -n ':has\(|has-\[|group-has-\[' <paths>
rg -n 'cursor-default|cursor-grab([[:space:]"'\''`}]|$)|cursor: grab([;[:space:]]|$)|cursor: default([;[:space:]]|$)' <paths>
```

The first scan catches presence data selectors that Tailwind v4 can usually
write as `data-ars-name:` or `group-data-ars-name:`. The second scan catches
descendant-state styling that should be justified or replaced with public-part
state attrs.
The cursor scan catches stale default/grab cursor rules; source templates
should normally use `pointer` for clickable rest state and `grabbing` for
active drag state.

If VS Code or the Tailwind language server still reports a "can be written as"
warning after the sweep, prefer the canonical spelling it suggests unless it
changes the selector semantics.
