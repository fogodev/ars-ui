---
adapter: dioxus
component: qr-code
category: specialized
source: components/specialized/qr-code.md
source_foundation: foundation/09-adapter-dioxus.md
---

# QrCode — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`QrCode`](../../components/specialized/qr-code.md) contract onto Dioxus `0.7.x`. The adapter preserves stateless QR matrix rendering, overlay support, accessible labeling, and optional download behavior while making matrix generation and download fallbacks explicit.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct QrCodeProps {
    pub value: String,
    #[props(optional)]
    pub error_correction: Option<QrErrorCorrection>,
    #[props(optional)]
    pub module_size: Option<f64>,
    #[props(optional)]
    pub quiet_zone: Option<usize>,
    #[props(optional)]
    pub foreground: Option<String>,
    #[props(optional)]
    pub background: Option<String>,
    #[props(optional)]
    pub overlay_src: Option<String>,
    #[props(optional)]
    pub overlay_size: Option<f64>,
    #[props(optional)]
    pub download_file_name: Option<String>,
}

#[component]
pub fn QrCode(props: QrCodeProps) -> Element
```

When `download_file_name` is `Some`, the adapter renders the optional `DownloadTrigger` and owns image export wiring.

## 3. Mapping to Core Component Contract

- Props parity: full parity with QR value, colors, sizing, error correction, overlay, and optional download affordance.
- Part parity: full parity with `Root`, `Frame`, `Pattern`, `Overlay`, and `DownloadTrigger`.
- Adapter additions: explicit matrix-generation fallback and download export policy.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                    | Notes                                        |
| --------------------- | --------- | ------------------------ | ------------- | ------------------------------ | -------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`             | Carries `role="img"` and resolved label.     |
| `Frame`               | optional  | `<div>`                  | adapter-owned | `api.frame_attrs()`            | Decorative wrapper when themed.              |
| `Pattern`             | required  | `<svg>`                  | adapter-owned | `api.pattern_attrs()`          | Canonical render target for SSR-safe output. |
| `Overlay`             | optional  | `<img>`                  | adapter-owned | `api.overlay_attrs()`          | Render only when `overlay_src` is present.   |
| `DownloadTrigger`     | optional  | `<button>`               | adapter-owned | `api.download_trigger_attrs()` | Render only when download is enabled.        |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                     | Adapter-owned attrs                   | Consumer attrs    | Merge order                   | Ownership notes                               |
| ----------------- | ------------------------------ | ------------------------------------- | ----------------- | ----------------------------- | --------------------------------------------- |
| `Root`            | role, label, size, scope, part | none beyond wrapper class merge       | decoration attrs  | root semantics and sizing win | consumer styling must not remove `role="img"` |
| `Pattern`         | QR matrix rendering attrs      | generated `<path>` or module children | none beyond style | generated pattern wins        | rendering stays adapter-owned                 |
| `DownloadTrigger` | button attrs and label         | click handler                         | decoration only   | label and handler win         | download trigger remains optional             |

## 6. Composition / Context Contract

`QrCode` is context-free. The download trigger, when present, is owned by the same component instance and does not publish separate context.

## 7. Prop Sync and Event Mapping

| Adapter prop           | Mode       | Sync trigger | Machine event / update path | Visible effect                             | Notes                           |
| ---------------------- | ---------- | ------------ | --------------------------- | ------------------------------------------ | ------------------------------- |
| visual props and value | controlled | rerender     | stateless recomputation     | rebuilds matrix, label, sizing, and colors | no post-mount sync store        |
| download enablement    | controlled | rerender     | structural branch           | toggles optional trigger                   | export path remains client-only |

## 8. Registration and Cleanup Contract

| Registered entity                 | Registration trigger             | Identity key       | Cleanup trigger                | Cleanup action    | Notes                       |
| --------------------------------- | -------------------------------- | ------------------ | ------------------------------ | ----------------- | --------------------------- |
| temporary object URL for download | user activates `DownloadTrigger` | component instance | download completion or cleanup | revoke object URL | client-only and short-lived |

## 9. Ref and Node Contract

| Target part / node | Ref required?                                   | Ref owner     | Node availability            | Composition rule | Notes                                         |
| ------------------ | ----------------------------------------------- | ------------- | ---------------------------- | ---------------- | --------------------------------------------- |
| `Pattern` root     | no for rendering; yes for client export helpers | adapter-owned | structural on SSR and client | no composition   | SVG export can serialize directly from markup |
| `DownloadTrigger`  | no                                              | adapter-owned | conditional                  | no composition   | only needs a click handler                    |

## 10. State Machine Boundary Rules

- machine-owned state: none; `QrCode` is stateless.
- adapter-local derived bookkeeping: optional export object URL only.
- forbidden local mirrors: do not keep a second matrix cache that can drift from props.

## 11. Callback Payload Contract

No public callbacks are required by this adapter surface.

## 12. Failure and Degradation Rules

| Condition                                         | Policy             | Notes                                                                         |
| ------------------------------------------------- | ------------------ | ----------------------------------------------------------------------------- |
| QR matrix generation fails for the supplied value | fail fast          | The adapter should surface a deterministic contract error during development. |
| browser download APIs unavailable                 | degrade gracefully | Render the code without `DownloadTrigger` behavior.                           |
| overlay image fails to load                       | degrade gracefully | Preserve QR pattern rendering without the overlay.                            |

## 13. Identity and Key Policy

The component owns no repeated registration beyond optional temporary download resources. Identity is the component instance.

## 14. SSR and Client Boundary Rules

- SSR renders `Root`, optional `Frame`, `Pattern`, and optional `Overlay`.
- Download export is client-only.
- The adapter should prefer SVG pattern rendering so SSR and hydration stay structurally identical.

## 15. Performance Constraints

- Rebuild the matrix only when QR-relevant props change.
- Prefer vector output over per-module DOM churn when possible.
- Revoke temporary export resources promptly.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type | Why it must exist first                                                          | Notes                                           |
| ------------------- | ----------- | --------------- | -------------------------------------------------------------------------------- | ----------------------------------------------- |
| QR matrix generator | required    | adapter helper  | The adapter owns encoding even though the core spec defines rendering semantics. | Wrap the chosen encoding crate behind a helper. |
| download helper     | recommended | platform helper | Keeps object URL creation and revocation localized.                              | Only needed when download is enabled.           |

## 17. Recommended Implementation Sequence

1. Encode the input into a QR matrix.
2. Render the root and pattern.
3. Add optional overlay support.
4. Add client-only download behavior.

## 18. Anti-Patterns

- Do not hide the QR image semantics behind decorative wrappers only.
- Do not make download support mandatory for basic rendering.
- Do not rely on canvas-only output for SSR.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the QR output is available without client-only APIs.
- Consumers may assume the root is labeled as an image.
- Consumers must not assume download support exists on every runtime.

## 20. Platform Support Matrix

| Capability / behavior     | Web          | Desktop       | Mobile        | SSR          | Notes                                                         |
| ------------------------- | ------------ | ------------- | ------------- | ------------ | ------------------------------------------------------------- |
| QR rendering and labeling | full support | full support  | full support  | full support | Prefer SVG output.                                            |
| overlay image             | full support | full support  | full support  | full support | Subject to image load success.                                |
| download trigger          | full support | fallback path | fallback path | client-only  | Desktop and mobile may use a platform-specific export helper. |

## 21. Debug Diagnostics and Production Policy

| Condition                   | Debug build behavior | Production behavior | Notes                                   |
| --------------------------- | -------------------- | ------------------- | --------------------------------------- |
| QR encoding helper fails    | fail fast            | fail fast           | Rendering contract cannot be satisfied. |
| download helper unavailable | debug warning        | degrade gracefully  | Hide or inert the trigger.              |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required?   | Responsibility                                    | Reused by                | Notes                          |
| ------------------ | ----------- | ------------------------------------------------- | ------------------------ | ------------------------------ |
| QR encoding helper | required    | converts the value into a matrix or SVG path data | `qr-code` only           | adapter-owned capability       |
| download helper    | recommended | serializes SVG and downloads it                   | download-capable visuals | revoke resources on completion |

## 23. Framework-Specific Behavior

Dioxus should keep QR encoding purely synchronous at render time, and any download serialization should run from the trigger event on the active platform without introducing background tasks that outlive the component.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct QrCodeSketchProps {
    pub value: String,
}

#[component]
pub fn QrCode(props: QrCodeSketchProps) -> Element {
    let api = qr_code::Api::new(&qr_code::Props { value: props.value, ..Default::default() });
    rsx! {
        div { ..api.root_attrs(),
            svg { ..api.pattern_attrs() }
        }
    }
}
```

## 25. Reference Implementation Skeleton

- Encode the current value through the QR helper.
- Render the semantic root and vector pattern.
- Gate download export behind a platform-aware branch.

## 26. Adapter Invariants

- The root always remains `role="img"` with a resolved label.
- The QR pattern remains renderable during SSR.
- Download support never blocks baseline rendering.

## 27. Accessibility and SSR Notes

Root labeling must remain present even when the encoded value is a URL or when an overlay is shown. Decorative frame and overlay content must not replace the root label.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: the adapter standardizes on SVG as the primary rendering target for hydration-safe output.

## 29. Test Scenarios

- baseline SVG QR rendering
- URL-specific accessible label
- overlay present and absent
- download trigger hidden or disabled when export support is unavailable

## 30. Test Oracle Notes

| Behavior         | Preferred oracle type | Notes                                                    |
| ---------------- | --------------------- | -------------------------------------------------------- |
| root semantics   | DOM attrs             | assert role and accessible label                         |
| QR structure     | rendered structure    | assert SVG root and matrix-derived children or path data |
| download cleanup | cleanup side effects  | assert temporary export resources are released           |

## 31. Implementation Checklist

- [ ] The root remains semantic and SSR-safe.
- [ ] Download behavior is optional and client-only or platform-gated.
- [ ] Overlay failure does not break QR rendering.
