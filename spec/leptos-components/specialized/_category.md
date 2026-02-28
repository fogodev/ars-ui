---
adapter: leptos
category: specialized
source_foundation: foundation/08-adapter-leptos.md
---

# Specialized Components — Leptos Adapter

These documents map the framework-agnostic specialized contracts in `spec/components/specialized/*` onto Leptos `0.8.x` APIs.

## Scope

- Core behavior, state machines, anatomy, accessibility, i18n, and form contracts remain defined by the framework-agnostic specialized specs.
- These Leptos adapter specs define the Leptos-facing component API, slot shape, `RwSignal<T>` usage for writable controlled values, lifecycle wiring, DOM ownership, semantic repair, SSR behavior, and browser fallback policy.
- Browser APIs such as Clipboard, EyeDropper, canvas drawing, drag-and-drop, timers, image measurement, and download flows are adapter concerns and must be restated explicitly in the component specs.
- Every specialized adapter spec is implementation-facing: it defines part mapping, attr ownership, prop sync, callbacks, cleanup ordering, ref requirements, platform support, diagnostics policy, and verification guidance.

## Conventions

- Use `RwSignal<T>` or `Option<RwSignal<T>>` for controlled values that the component mutates after mount.
- Use plain props for static configuration and `Children` or explicit slots only when consumer-owned structure is part of the adapter contract.
- Pointer-heavy widgets such as `ColorArea`, `ColorSlider`, `ColorWheel`, `AngleSlider`, `SignaturePad`, and `ImageCropper` must own drag-session listeners, geometry reads, and cleanup in client-only effects.
- Canvas and image-based widgets must keep node ownership explicit, defer measurement until mount, and document `touch-action: none` or equivalent semantic repair where required by the core contract.
- Clipboard, timer, EyeDropper, download, and drag-and-drop behavior must pair every runtime capability requirement with an explicit fallback or production policy.
- Color components form one composition cluster. Standalone color primitives must document how they operate independently and how they compose inside `ColorPicker`.
- Hidden form inputs, live regions, and browser capability probes are adapter-owned helper patterns that must not remain implied by the framework-agnostic spec.
- Canonical implementation sketches remain illustrative; the numbered contract sections and `Adapter Invariants` are normative.

### Final Section Structure

Every specialized adapter component spec in this tree uses this final section order:

1. `Purpose and Adapter Scope`
2. `Public Adapter API`
3. `Mapping to Core Component Contract`
4. `Part Mapping`
5. `Attr Merge and Ownership Rules`
6. `Composition / Context Contract`
7. `Prop Sync and Event Mapping`
8. `Registration and Cleanup Contract`
9. `Ref and Node Contract`
10. `State Machine Boundary Rules`
11. `Callback Payload Contract`
12. `Failure and Degradation Rules`
13. `Identity and Key Policy`
14. `SSR and Client Boundary Rules`
15. `Performance Constraints`
16. `Implementation Dependencies`
17. `Recommended Implementation Sequence`
18. `Anti-Patterns`
19. `Consumer Expectations and Guarantees`
20. `Platform Support Matrix`
21. `Debug Diagnostics and Production Policy`
22. `Shared Adapter Helper Notes`
23. `Framework-Specific Behavior`
24. `Canonical Implementation Sketch`
25. `Reference Implementation Skeleton`
26. `Adapter Invariants`
27. `Accessibility and SSR Notes`
28. `Parity Summary and Intentional Deviations`
29. `Test Scenarios`
30. `Test Oracle Notes`
31. `Implementation Checklist`

### Shared Terminology

- `registration`: adapter-owned mount or unmount bookkeeping for descendants, listeners, timers, hidden inputs, live regions, upload handles, or measurement targets.
- `structural node`: a rendered node whose presence or identity is part of the public adapter contract.
- `live handle`: a runtime node reference used for DOM measurement, canvas painting, pointer capture, or imperative browser APIs.
- `fallback path`: the documented degraded behavior used when a browser capability or runtime primitive is unavailable.
- `semantic repair`: adapter-owned roles, `aria-*`, ids, `touch-action`, hidden-input wiring, or event normalization added so the host output still satisfies the core contract.

### Authoring Lint Checklist

- [ ] No specialized adapter-owned behavior remains only in the framework-agnostic spec.
- [ ] Browser capability requirements are paired with explicit fallback or production behavior.
- [ ] Pointer, timer, and canvas cleanup are documented where the component owns them.
- [ ] Hidden input and live-region behavior are documented where the component participates in forms or announcements.
- [ ] Color component composition and standalone use are both explicit where relevant.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.

## Specialized Index

- [AngleSlider](angle-slider.md)
- [Clipboard](clipboard.md)
- [ColorArea](color-area.md)
- [ColorField](color-field.md)
- [ColorPicker](color-picker.md)
- [ColorSlider](color-slider.md)
- [ColorSwatch](color-swatch.md)
- [ColorSwatchPicker](color-swatch-picker.md)
- [ColorWheel](color-wheel.md)
- [ContextualHelp](contextual-help.md)
- [FileUpload](file-upload.md)
- [ImageCropper](image-cropper.md)
- [QrCode](qr-code.md)
- [SignaturePad](signature-pad.md)
- [Timer](timer.md)
