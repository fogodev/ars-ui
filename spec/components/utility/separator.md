---
component: Separator
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    radix-ui: Separator
    react-aria: Separator
---

# Separator

A horizontal or vertical dividing line used to group and visually separate content.

## 1. API

### 1.1 Props

```rust
use ars_i18n::Orientation;

/// Props for the `Separator` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The orientation of the separator. Defaults to `Orientation::Horizontal`
    /// via `Orientation`'s own `Default` impl.
    pub orientation: Orientation,
    /// Whether the separator is purely decorative and hidden from the
    /// accessibility tree.
    pub decorative: bool,
}

impl Props {
    /// Returns fresh props with the documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the layout orientation of the separator.
    #[must_use]
    pub const fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets whether the separator is purely decorative.
    #[must_use]
    pub const fn decorative(mut self, value: bool) -> Self {
        self.decorative = value;
        self
    }
}
```

### 1.2 Connect / API

```rust
/// DOM parts of the `Separator` component.
#[derive(ComponentPart)]
#[scope = "separator"]
pub enum Part {
    /// The root element. See §2.1 for adapter element-type selection
    /// (`<hr>` vs `<div>`).
    Root,
}

/// The API for the `Separator` component.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Creates a new `Api` instance from the given props.
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns a reference to the underlying `Props`.
    ///
    /// Adapters typically read individual fields through the dedicated
    /// accessors (`id`, `orientation`, `decorative`); this method is the
    /// escape hatch for when the full struct is needed (e.g., to clone it
    /// into a fresh `Api` for a re-render).
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns the layout orientation of the separator.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.props.orientation
    }

    /// Returns whether the separator is purely decorative.
    #[must_use]
    pub const fn decorative(&self) -> bool {
        self.props.decorative
    }

    /// Returns the attributes for the root element.
    ///
    /// Semantic separators get `role="separator"` plus `aria-orientation`
    /// matching the layout axis, and `data-ars-orientation` for styling.
    /// Decorative separators get `role="none"` (the modern WAI-ARIA 1.2
    /// preferred form, synonymous with `role="presentation"`); they omit
    /// `aria-orientation` and the `data-ars-orientation` styling hook,
    /// because a decorative separator is invisible to assistive technology
    /// and component-managed orientation styling is not applied.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);

        if self.props.decorative {
            // Decorative separators are removed from the accessibility tree
            // via `role="none"`. No `aria-hidden` (redundant for an element
            // with no children) and no `data-ars-orientation` (decorative
            // separators do not participate in component orientation styling).
            p.set(HtmlAttr::Role, "none");
        } else {
            let orientation_str = match self.props.orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            };
            p.set(HtmlAttr::Data("ars-orientation"), orientation_str);
            p.set(HtmlAttr::Role, "separator");
            p.set(HtmlAttr::Aria(AriaAttr::Orientation), orientation_str);
        }

        p
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Separator
└── Root  <hr> or <div>  data-ars-scope="separator" data-ars-part="root"
                         role="separator" | role="none"
```

| Part | Element          | Key Attributes                                                                            |
| ---- | ---------------- | ----------------------------------------------------------------------------------------- |
| Root | `<hr>` / `<div>` | `data-ars-scope="separator"`, `data-ars-part="root"`, `role`, optional `aria-orientation` |

### 2.1 Element Choice

The agnostic-core does not prescribe an element type — that is an adapter
concern, documented in `spec/leptos-components/utility/separator.md` and
`spec/dioxus-components/utility/separator.md`. Adapters render `<hr>` for
content separators (between paragraphs, sections) and `<div>` for menu,
toolbar, or listbox separators where `<hr>` is semantically inappropriate.
When `decorative` is true, either element is acceptable.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Value                                                  |
| ------------------ | ------------------------------------------------------ |
| Role               | `separator` (semantic) or `none` (decorative)          |
| `aria-orientation` | `"horizontal"` / `"vertical"` (omitted for decorative) |

- Semantic `<hr>` has implicit `role="separator"` in HTML. When using a `<div>`, `role="separator"` must be set explicitly.
- Decorative separators (e.g., in a dropdown menu between groups that are already labeled) use `role="none"` (the WAI-ARIA 1.2 preferred form; synonymous with the older `role="presentation"`). `aria-hidden="true"` is **not** added: the separator has no children to suppress, and `role="none"` already removes it from the accessibility tree.
- `aria-orientation` is optional for horizontal separators (the default) but recommended for vertical ones; the agnostic-core sets it explicitly for both axes for cross-AT robustness.

## 4. Internationalization

- Separators have no text content and require no localization.
- In RTL layouts, vertical separators remain visually unchanged. The content on either side of the separator reflows according to the document direction.

## 5. Library Parity

> Compared against: Radix UI (`Separator`), React Aria (`Separator`).

### 5.1 Props

| Feature      | ars-ui        | Radix UI      | React Aria    | Notes                                                    |
| ------------ | ------------- | ------------- | ------------- | -------------------------------------------------------- |
| Orientation  | `orientation` | `orientation` | `orientation` | All libraries                                            |
| Decorative   | `decorative`  | `decorative`  | --            | Radix and ars-ui; RA uses role="presentation" implicitly |
| Element type | --            | --            | `elementType` | RA allows overriding the HTML element                    |

**Gaps:** None. React Aria's `elementType` is handled by ars-ui's adapter element choice (section 2.1).

### 5.2 Role token for decorative separators

| Library    | Decorative role       | Notes                                                                                                                                             |
| ---------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| ars-ui     | `role="none"`         | WAI-ARIA 1.2 preferred form. No `aria-hidden` (redundant for an element with no children).                                                        |
| Radix UI   | `role="none"`         | Recent versions (Radix UI ≥ 1.x) emit `role="none"`; earlier versions used `role="presentation"`.                                                 |
| React Aria | `role="presentation"` | Older WAI-ARIA term, semantically equivalent to `none`. Browser/AT support for `none` is universal in modern environments; ars-ui prefers `none`. |

### 5.3 Anatomy

| Part | ars-ui | Radix UI | React Aria  | Notes                      |
| ---- | ------ | -------- | ----------- | -------------------------- |
| Root | `Root` | `Root`   | `Separator` | All libraries; single-part |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria exposes `elementType` as a prop; ars-ui handles element choice at the adapter level (section 2.1). React Aria also uses the older `role="presentation"` token for decorative separators where ars-ui (and modern Radix) use `role="none"`.
- **Recommended additions:** None.
