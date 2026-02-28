---
adapter: dioxus
component: image-cropper
category: specialized
source: components/specialized/image-cropper.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ImageCropper — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ImageCropper`](../../components/specialized/image-cropper.md) contract onto Dioxus `0.7.x`. The adapter preserves crop-region dragging and resizing, zoom/rotation controls, keyboard nudging, image measurement, and client-only export behavior.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ImageCropperProps {
    pub src: String,
    #[props(optional)]
    pub crop: Option<Signal<CropArea>>,
    #[props(optional)]
    pub default_crop: Option<CropArea>,
    #[props(optional)]
    pub zoom: Option<Signal<f64>>,
    #[props(optional)]
    pub rotation: Option<Signal<f64>>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub circular: Option<bool>,
    #[props(optional)]
    pub show_reset: Option<bool>,
}

#[component]
pub fn ImageCropper(props: ImageCropperProps) -> Element
```

The adapter renders the image, overlay, crop area, resize handles, optional zoom/rotation controls, and reset trigger.

## 3. Mapping to Core Component Contract

- Props parity: full parity with crop binding, transform controls, disabled state, and reset behavior.
- Part parity: full parity with `Root`, `Image`, `Overlay`, `CropArea`, `Grid`, repeated `Handle`, optional sliders, `ResetTrigger`, and `Label`.
- Adapter additions: explicit image measurement, `touch-action: none`, and export fallback policy.

## 4. Part Mapping

Image, overlay, crop area, rule-of-thirds grid, repeated handles, optional zoom/rotation sliders, reset trigger, and label are all adapter-owned. `CropArea` and `Handle` are the interactive geometry surfaces.

## 5. Attr Merge and Ownership Rules

`Root` keeps application semantics, `CropArea` and `Handle` keep `touch-action: none`, and handle labels always win. Consumer decoration must not remove resize or keyboard semantics.

## 6. Composition / Context Contract

`ImageCropper` is a single integrated surface. Consumers do not render crop handles or transform controls separately in the base adapter contract.

## 7. Prop Sync and Event Mapping

Controlled crop, zoom, and rotation sync flow through the writable signals. Pointer drag on `CropArea` moves the region; pointer drag on a `Handle` resizes it. Keyboard events on `CropArea` nudge or transform according to the core shortcut contract.

## 8. Registration and Cleanup Contract

Register one active drag or resize session plus any image-load and export resources per instance. Cleanup must cancel global pointer listeners, clear temporary export resources, and discard stale image measurements on unmount.

## 9. Ref and Node Contract

The source image and crop root need live handles for measurement after mount. Interactive handles remain adapter-owned and do not expose consumer refs.

## 10. State Machine Boundary Rules

- machine-owned state: crop area, zoom, rotation, flip state, focused handle, and disabled semantics.
- adapter-local derived bookkeeping: image rect, active drag session, and export helper state only.
- forbidden local mirrors: do not keep a second crop rectangle outside the machine bindings.

## 11. Callback Payload Contract

No dedicated callback is required beyond the bindable crop and transform values.

## 12. Failure and Degradation Rules

If image measurement fails, degrade gracefully by rendering the static image and disabling interactive crop movement until measurement succeeds. Export remains optional and client-only.

## 13. Identity and Key Policy

Resize handles use stable positional identities. Crop-region identity is the component instance plus the machine-owned crop area.

## 14. SSR and Client Boundary Rules

SSR renders the structural image-cropper surface without live measurement. Pointer interactions, image rect measurement, and export are client-only.

## 15. Performance Constraints

Keep at most one drag or resize session active. Recompute handle positions only when crop geometry or transforms change.

## 16. Implementation Dependencies

| Dependency               | Required? | Dependency type | Why it must exist first                      | Notes                            |
| ------------------------ | --------- | --------------- | -------------------------------------------- | -------------------------------- |
| image-measurement helper | required  | geometry helper | crop math depends on the rendered image rect | adapter-owned                    |
| drag-session helper      | required  | pointer helper  | move and resize cleanup must stay consistent | shared with interactive surfaces |

## 17. Recommended Implementation Sequence

1. Render image, overlay, crop area, and handles.
2. Measure the image after mount.
3. Add crop move and handle resize behavior with cleanup.
4. Add optional transform controls and reset trigger.

## 18. Anti-Patterns

- Do not allow browser pan/zoom to compete with crop gestures on interactive surfaces.
- Do not keep stale image geometry after src or layout changes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume crop movement and resize are disabled until measurement is available.
- Consumers may assume reset returns to the documented default crop.
- Consumers must not assume export works during SSR.

## 20. Platform Support Matrix

| Capability / behavior                   | Web           | Desktop       | Mobile        | SSR            | Notes                                            |
| --------------------------------------- | ------------- | ------------- | ------------- | -------------- | ------------------------------------------------ |
| structural cropper surface              | full support  | full support  | full support  | full support   | no live geometry on SSR                          |
| crop dragging, resizing, and transforms | full support  | full support  | full support  | client-only    | requires image measurement and pointer listeners |
| export helpers                          | fallback path | fallback path | fallback path | not applicable | optional adapter-owned capability                |

## 21. Debug Diagnostics and Production Policy

Missing image measurement after load is a debug warning and graceful interaction fallback. Multiple active drag sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use one measurement helper and one pointer-session helper so crop movement, handle resize, and transform cleanup stay aligned.

## 23. Framework-Specific Behavior

Dioxus should keep image measurement and pointer cleanup in client effects or instance-local guards and ensure `touch-action: none` is applied to `CropArea` and every resize handle.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ImageCropperSketchProps {
    pub src: String,
}

#[component]
pub fn ImageCropper(props: ImageCropperSketchProps) -> Element {
    let machine = use_machine::<image_cropper::Machine>(image_cropper::Props { src: props.src, ..Default::default() });
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Bind crop and transform values.
- Measure the image after mount.
- Render crop area and handles from machine geometry.
- Clean up drag sessions and temporary export resources eagerly.

## 26. Adapter Invariants

- `CropArea` and `Handle` always retain `touch-action: none`.
- At most one drag or resize session exists at a time.
- Image geometry changes always invalidate stale crop math.

## 27. Accessibility and SSR Notes

The root retains application semantics and the crop region remains keyboard reachable. Decorative image overlays stay `aria-hidden`.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- image measurement gates interaction
- crop move and handle resize cleanup
- keyboard nudge and transform shortcuts
- reset trigger restores default crop
- unsupported export path does not break editing

## 30. Test Oracle Notes

| Behavior              | Preferred oracle type | Notes                                              |
| --------------------- | --------------------- | -------------------------------------------------- |
| interactive semantics | DOM attrs             | assert application and handle-label attrs          |
| geometry behavior     | rendered behavior     | assert crop box updates after representative drags |
| cleanup               | cleanup side effects  | assert pointer listeners are removed               |

## 31. Implementation Checklist

- [ ] Image measurement is required before interactive drag math.
- [ ] `touch-action: none` is applied to the interactive surfaces.
- [ ] Drag and export cleanup are explicit.
