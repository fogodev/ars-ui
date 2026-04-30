---
adapter: dioxus
component: signature-pad
category: specialized
source: components/specialized/signature-pad.md
source_foundation: foundation/09-adapter-dioxus.md
---

# SignaturePad — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`SignaturePad`](../../components/specialized/signature-pad.md) contract onto Dioxus `0.7.x`. The adapter preserves canvas-based stroke capture, undo and clear controls, guide rendering, hidden-input submission, and high-DPI canvas lifecycle rules.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct SignaturePadProps {
    #[props(optional)]
    pub value: Option<Signal<SignatureData>>,
    #[props(optional)]
    pub default_value: Option<SignatureData>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub readonly: Option<bool>,
    #[props(optional)]
    pub show_clear: Option<bool>,
    #[props(optional)]
    pub show_undo: Option<bool>,
    #[props(optional)]
    pub show_guide: Option<bool>,
    #[props(optional)]
    pub label: Option<String>,
}

#[component]
pub fn SignaturePad(props: SignaturePadProps) -> Element
```

The adapter renders the root, label, canvas, optional guide, clear and undo triggers, and hidden input.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable signature data, disabled/read-only state, guide visibility, and action triggers.
- Part parity: full parity with `Root`, `Label`, `Canvas`, `Guide`, `ClearTrigger`, `UndoTrigger`, and `HiddenInput`.
- Adapter additions: explicit high-DPI canvas sizing, pointer capture cleanup, and export fallback notes.

## 4. Part Mapping

Canvas, guide, action triggers, and hidden input are adapter-owned. `Canvas` is the only drawing surface and owns the imperative rendering lifecycle.

## 5. Attr Merge and Ownership Rules

Canvas semantics, `touch-action: none`, disabled/read-only attrs, and action-trigger labels always win. Consumer decoration must not remove the interactive canvas or hidden input.

## 6. Composition / Context Contract

`SignaturePad` is a single integrated surface. Consumers do not render canvas or action triggers separately in the base contract.

## 7. Prop Sync and Event Mapping

Controlled signature sync flows through the writable signal. Pointer or touch input on `Canvas` dispatches draw-start, draw-move, and draw-end events. Clear and undo buttons dispatch the corresponding machine events.

## 8. Registration and Cleanup Contract

Register one active drawing session, canvas-render context ownership, and any high-DPI resize bookkeeping per instance. Cleanup must cancel global pointer listeners, release stale drawing sessions, and discard transient rendering resources on unmount.

## 9. Ref and Node Contract

`Canvas` requires a live node after mount for sizing, coordinate transforms, and drawing. No consumer ref composition is part of the base surface.

## 10. State Machine Boundary Rules

- machine-owned state: stroke list, drawing state, disabled/read-only semantics, and undo/clear availability.
- adapter-local derived bookkeeping: canvas context handle, device-pixel-ratio sizing state, and active pointer session only.
- forbidden local mirrors: do not keep a second stroke list outside the machine binding.

## 11. Callback Payload Contract

No dedicated callback is required beyond the bindable signature data and hidden input.

## 12. Failure and Degradation Rules

If canvas APIs are unavailable, degrade gracefully by rendering the labeled static surface and disabling interactive drawing. Export remains optional and client-only.

## 13. Identity and Key Policy

Stroke identity stays machine-owned. Clear and undo actions always target the current instance’s stroke list only.

## 14. SSR and Client Boundary Rules

SSR renders the structural signature-pad surface but performs no canvas drawing. Canvas sizing, drawing, and export are client-only.

## 15. Performance Constraints

Keep at most one active drawing session. Batch canvas redraw work so high-frequency pointer moves do not allocate new structural nodes.

## 16. Implementation Dependencies

| Dependency             | Required? | Dependency type   | Why it must exist first                              | Notes                            |
| ---------------------- | --------- | ----------------- | ---------------------------------------------------- | -------------------------------- |
| canvas helper          | required  | imperative helper | canvas sizing and redraw depend on a live 2D context | adapter-owned                    |
| pointer-session helper | required  | pointer helper    | draw cleanup must stay consistent                    | shared with interactive surfaces |

## 17. Recommended Implementation Sequence

1. Render root, label, canvas, guide, triggers, and hidden input.
2. Size the canvas after mount with device-pixel-ratio awareness.
3. Add drawing sessions and cleanup.
4. Add clear and undo actions.

## 18. Anti-Patterns

- Do not allow browser panning on the drawing canvas.
- Do not store stroke rendering state separately from the machine’s stroke data.

## 19. Consumer Expectations and Guarantees

- Consumers may assume undo removes the latest stroke only.
- Consumers may assume the hidden input contains the serialized signature.
- Consumers must not assume drawing works during SSR.

## 20. Platform Support Matrix

| Capability / behavior         | Web           | Desktop       | Mobile        | SSR            | Notes                             |
| ----------------------------- | ------------- | ------------- | ------------- | -------------- | --------------------------------- |
| structural signature surface  | full support  | full support  | full support  | full support   | no live drawing on SSR            |
| canvas drawing and undo/clear | full support  | full support  | full support  | client-only    | requires canvas and pointer APIs  |
| export helpers                | fallback path | fallback path | fallback path | not applicable | optional adapter-owned capability |

## 21. Debug Diagnostics and Production Policy

Missing canvas context after mount is a debug warning and graceful interaction fallback. Multiple active drawing sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use one canvas helper for device-pixel-ratio sizing and redraws plus one pointer-session helper for drawing cleanup.

## 23. Framework-Specific Behavior

Dioxus should size the canvas after mount, keep `touch-action: none` on the canvas, and cancel pointer listeners before any redraw resource is discarded.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn SignaturePad() -> Element {
    let machine = use_machine::<signature_pad::Machine>(signature_pad::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Bind signature data.
- Mount and size the canvas after mount.
- Render strokes from machine state only.
- Clean up drawing sessions and redraw resources eagerly.

## 26. Adapter Invariants

- `Canvas` always retains `touch-action: none`.
- At most one drawing session exists at a time.
- Canvas redraws derive from machine stroke state only.

## 27. Accessibility and SSR Notes

The canvas remains the primary interactive surface and should switch to visual-only semantics when disabled or read-only, matching the core contract.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- drawing session start/move/end cleanup
- undo and clear behavior
- high-DPI canvas sizing after mount
- hidden input reflects serialized signature
- canvas unavailable fallback

## 30. Test Oracle Notes

| Behavior         | Preferred oracle type | Notes                                          |
| ---------------- | --------------------- | ---------------------------------------------- |
| canvas semantics | DOM attrs             | assert role and action-trigger labels          |
| drawing cleanup  | cleanup side effects  | assert no stale pointer listeners              |
| serialization    | rendered behavior     | assert hidden-input value changes with strokes |

## 31. Implementation Checklist

- [ ] Canvas sizing is mount-only and DPR-aware.
- [ ] `touch-action: none` remains on the canvas.
- [ ] Drawing cleanup is explicit and singular.
