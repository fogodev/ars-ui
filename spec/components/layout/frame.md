---
component: Frame
category: layout
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: [aspect-ratio]
references: {}
---

# Frame

`Frame` is a stateless layout wrapper around `<iframe>` elements. It provides a declarative API for sandboxing, permissions policy, lazy loading, and optional responsive aspect ratio sizing. There is no state machine; there is no interactive behaviour.

## 1. API

### 1.1 Props

```rust
/// Loading strategy for the iframe content.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadingStrategy {
    /// Load the iframe immediately (default browser behavior).
    #[default]
    Eager,
    /// Defer loading until the iframe is near the viewport.
    Lazy,
}

/// The props for the Frame component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// URL of the content to embed.
    pub src: String,
    /// Accessible title for the iframe (required).
    /// Screen readers announce this as the frame's accessible name.
    pub title: String,
    /// Sandbox restrictions. Space-separated tokens (e.g., "allow-scripts allow-same-origin").
    /// When `Some("")`, applies maximum sandboxing (no tokens).
    /// When `None`, no sandbox attribute is set (unrestricted).
    pub sandbox: Option<String>,
    /// Permissions policy for cross-origin features (e.g., "camera; microphone").
    pub allow: Option<String>,
    /// Loading strategy. `Lazy` defers loading via `loading="lazy"`.
    pub loading: LoadingStrategy,
    /// When set, wraps the iframe in an aspect-ratio container using the
    /// padding-top technique. Value is width/height (e.g., 16.0/9.0).
    pub aspect_ratio: Option<f64>,
    /// Explicit width (CSS value, e.g., "100%", "640px"). Defaults to "100%".
    pub width: String,
    /// Explicit height (CSS value, e.g., "480px", "auto"). Defaults to "auto".
    pub height: String,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            id: String::new(),
            src: String::new(),
            title: String::new(),
            sandbox: None,
            allow: None,
            loading: LoadingStrategy::Eager,
            aspect_ratio: None,
            width: "100%".to_string(),
            height: "auto".to_string(),
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "frame"]
pub enum Part {
    Root,
    Iframe,
}

pub struct Api {
    props: Props,
}

impl Api {
    pub fn new(props: Props) -> Self { Self { props } }

    /// Attributes for the outer container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(ratio) = self.props.aspect_ratio {
            let padding = (1.0 / ratio) * 100.0;
            attrs.set_style(CssProperty::Position, "relative");
            attrs.set_style(CssProperty::Width, &self.props.width);
            attrs.set_style(CssProperty::PaddingTop, format!("{:.4}%", padding));
        }
        attrs
    }

    /// Attributes for the iframe element.
    pub fn iframe_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Iframe.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Src, &self.props.src);
        attrs.set(HtmlAttr::Title, &self.props.title);
        if let Some(sandbox) = &self.props.sandbox {
            attrs.set(HtmlAttr::Sandbox, sandbox);
        }
        if let Some(allow) = &self.props.allow {
            attrs.set(HtmlAttr::Allow, allow);
        }
        match self.props.loading {
            LoadingStrategy::Lazy => { attrs.set(HtmlAttr::Loading, "lazy"); }
            LoadingStrategy::Eager => {}
        }
        if self.props.aspect_ratio.is_some() {
            attrs.set_style(CssProperty::Position, "absolute");
            attrs.set_style(CssProperty::Inset, "0");
            attrs.set_style(CssProperty::Width, "100%");
            attrs.set_style(CssProperty::Height, "100%");
            attrs.set_style(CssProperty::Border, "0");
        } else {
            attrs.set_style(CssProperty::Width, &self.props.width);
            attrs.set_style(CssProperty::Height, &self.props.height);
            attrs.set_style(CssProperty::Border, "0");
        }
        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Iframe => self.iframe_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Frame
├── Root     <div>     data-ars-scope="frame" data-ars-part="root"
└── Iframe   <iframe>  data-ars-scope="frame" data-ars-part="iframe"
```

| Part   | Element    | Key Attributes                                               |
| ------ | ---------- | ------------------------------------------------------------ |
| Root   | `<div>`    | Aspect-ratio sizing via inline `padding-top` when configured |
| Iframe | `<iframe>` | `src`, `title`, `sandbox`, `allow`, `loading`                |

Both parts are required. Root is only visually relevant when `aspect_ratio` is set; otherwise it acts as a transparent wrapper.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property | Value                                                                 |
| -------- | --------------------------------------------------------------------- |
| Role     | None added — `<iframe>` has an implicit `document` role               |
| `title`  | Required. Screen readers announce this as the frame's accessible name |

- **`title` is required.** Omitting `title` makes the iframe invisible to screen readers.
- **Keyboard.** Users can Tab into the iframe. Once inside, keyboard interaction is governed by the embedded content, not ars-ui.

## 4. Internationalization

- The `title` prop is consumer-provided and must be localized by the consumer.
- No component-generated text exists; no `Messages` struct is needed.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
