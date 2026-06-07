# Widgets Visual Review Checklist

Use this checklist with the local widgets page and the counterpart docs open in
the browser.

## Counterpart Comparison

- [ ] React Aria / React Spectrum page inspected when available.
- [ ] Ark UI / Chakra UI fallback inspected when React Aria / Spectrum does not
      cover the component or feature axis.
- [ ] Radix UI / shadcn/ui fallback inspected when earlier counterparts do not
      cover the component or feature axis.
- [ ] Simplest counterpart example mapped to our first demo section.
- [ ] Advanced counterpart examples mapped to supported demo sections.
- [ ] Intentional visual/content differences documented.
- [ ] `playwright-cli` artifact paths recorded for reference and local pages.
- [ ] Every supported reference outcome appears in the final parity matrix with
      local widget evidence or a documented reason widgets are not the proof
      surface.
- [ ] Reference API shape was reviewed as outcome evidence only; local widget
      proof uses idiomatic ars-ui APIs instead of copying React/TypeScript API
      structure.
- [ ] No supported outcome depends on raw native controls, sibling error UI, or
      duplicated component policy in the widget demo.
- [ ] The widget behaves like a consumer application: it owns sample data,
      controlled values, callback sinks, consumer-owned copy, routing, layout,
      and styling only.
- [ ] The widget does not implement component-owned validation, ARIA
      relationships, keyboard/focus behavior, selection, drag/drop, loading,
      layout, popup state, or localized-message policy.

## Visible States

- [ ] Selected state is visible across the full item area.
- [ ] Checked and indeterminate states are visible where supported.
- [ ] Focus-visible state is clear.
- [ ] Hover/pressed/active feedback is visible where supported.
- [ ] Disabled state is visually distinct.
- [ ] Readonly and required states are visually distinct where supported.
- [ ] Invalid/error state is visible and announced where supported.
- [ ] Form submit and reset behavior is visible where supported.
- [ ] Empty state has clear text.
- [ ] Loading state has visible status, not only a sentinel.
- [ ] Links and actions are distinguishable.
- [ ] Drag image represents the dragged item set.
- [ ] Drop target highlights the full placement area.
- [ ] Grid/list layout differences are obvious.

## Stability

- [ ] Text fits on mobile and desktop.
- [ ] Controls do not shift after selection or loading state changes.
- [ ] Computed dimensions, colors, opacity, cursor, and visibility were checked
      for every supported visible state.
- [ ] Popups/overlays anchor to their trigger.
- [ ] Scrollable areas show affordances.
- [ ] Browser console is clean after page load and representative interactions.
- [ ] Locale switch checked when the widget shell supports locales; labels,
      descriptions, placeholders, validation messages, status text, button text,
      and announcements update for component-owned text.
