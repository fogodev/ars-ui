---
component: Landmark
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  react-aria: Landmark
---

# Landmark

Landmark provides semantic landmark regions for page structure, mapping to ARIA landmark roles and their corresponding HTML5 elements.

## 1. API

### 1.1 Props

```rust
/// The role of the landmark region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// <header>
    Banner,
    /// <nav>
    Navigation,
    /// <main>
    Main,
    /// <aside>
    Complementary,
    /// <footer>
    ContentInfo,
    /// <search> (HTML5.2) or <div role="search">
    Search,
    /// <form> (only a landmark when it has an accessible name)
    Form,
    /// <section> (only a landmark when it has an accessible name)
    Region,
}

impl Role {
    /// Returns the WAI-ARIA role string for use in `role` attribute.
    pub const fn aria_role(self) -> &'static str {
        match self {
            Role::Banner => "banner",
            Role::Navigation => "navigation",
            Role::Main => "main",
            Role::Complementary => "complementary",
            Role::ContentInfo => "contentinfo",
            Role::Search => "search",
            Role::Form => "form",
            Role::Region => "region",
        }
    }

    /// Returns `true` when the role must have an accessible name to be
    /// recognized as a landmark by assistive technology.
    pub const fn requires_accessible_name(self) -> bool {
        matches!(self, Role::Form | Role::Region)
    }
}

/// Props for the `Landmark` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The role of the landmark region.
    pub role: Role,
    /// Optional ID of an element that labels this landmark.
    /// Per WAI-ARIA, `aria-labelledby` and `aria-label` MUST NOT be set
    /// simultaneously. When `labelledby_id` is set, it takes precedence
    /// over `messages.label` and emits `aria-labelledby` instead of `aria-label`.
    pub labelledby_id: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            role: Role::Region,
            labelledby_id: None,
        }
    }
}

impl Props {
    pub fn new() -> Self { Self::default() }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
    pub const fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    pub fn labelledby_id(mut self, id: impl Into<String>) -> Self {
        self.labelledby_id = Some(id.into());
        self
    }
    pub fn unlabelled(mut self) -> Self {
        self.labelledby_id = None;
        self
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "landmark"]
pub enum Part {
    Root,
}

pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns `true` when adapters should prefer their generic fallback
    /// element for the default rendering path.
    ///
    /// Adapters can still call `root_attrs(false)` for any role when they
    /// render an explicit-role fallback element.
    pub fn prefers_generic_fallback_element(&self) -> bool {
        matches!(self.props.role, Role::Search)
    }

    /// Returns `true` when the landmark has an accessible name from
    /// `aria-labelledby` or from the resolved localized label message.
    pub fn has_accessible_name(&self) -> bool {
        self.props.labelledby_id.is_some() || !(self.messages.label)(&self.locale).is_empty()
    }

    /// Returns `true` when this API should emit the debug warning for a
    /// required but missing accessible name.
    pub fn missing_accessible_name_warning_needed(&self) -> bool {
        self.props.role.requires_accessible_name() && !self.has_accessible_name()
    }

    /// Returns the root attributes.
    ///
    /// When the adapter renders a native landmark element, the explicit
    /// `role` attribute is redundant and should be omitted. Set
    /// `is_native_landmark_element` to `false` when rendering a generic fallback
    /// element such as `<div>`.
    pub fn root_attrs(&self, is_native_landmark_element: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, &self.props.id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        // Only set role explicitly when using a generic fallback element;
        // semantic HTML elements carry implicit roles.
        if !is_native_landmark_element {
            attrs.set(HtmlAttr::Role, self.props.role.aria_role());
        }

        // Per WAI-ARIA, aria-labelledby and aria-label MUST NOT be set
        // simultaneously. When labelledby_id is set, it takes precedence.
        if let Some(ref id) = self.props.labelledby_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), id);
        } else {
            let label = (self.messages.label)(&self.locale);
            if !label.is_empty() {
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
            }

            if label.is_empty() && self.missing_accessible_name_warning_needed() {
                warn_missing_accessible_name(self.props.role);
            }
        }

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(true),
        }
    }
}

#[cfg(feature = "debug")]
fn warn_missing_accessible_name(role: Role) {
    if role.requires_accessible_name() {
        log::warn!(
            "landmark: role '{}' requires an accessible name (aria-label or aria-labelledby)",
            role.aria_role()
        );
    }
}

#[cfg(all(
    debug_assertions,
    not(feature = "debug"),
    feature = "std",
    not(all(
        target_arch = "wasm32",
        not(any(target_os = "emscripten", target_os = "wasi"))
    ))
))]
fn warn_missing_accessible_name(role: Role) {
    if role.requires_accessible_name() {
        eprintln!(
            "landmark: role '{}' requires an accessible name (aria-label or aria-labelledby)",
            role.aria_role()
        );
    }
}

#[cfg(not(any(
    feature = "debug",
    all(
        debug_assertions,
        feature = "std",
        not(all(
            target_arch = "wasm32",
            not(any(target_os = "emscripten", target_os = "wasi"))
        ))
    )
)))]
const fn warn_missing_accessible_name(_role: Role) {}
```

## 2. Anatomy

```text
Landmark
└── Root  (<header> | <nav> | <main> | <aside> | <footer> | <search> | <form> | <section> | <div role="...">)
    └── {children}
```

The root element is chosen based on `role`:

| Role            | HTML Element | Fallback                     |
| --------------- | ------------ | ---------------------------- |
| `Banner`        | `<header>`   | `<div role="banner">`        |
| `Navigation`    | `<nav>`      | `<div role="navigation">`    |
| `Main`          | `<main>`     | `<div role="main">`          |
| `Complementary` | `<aside>`    | `<div role="complementary">` |
| `ContentInfo`   | `<footer>`   | `<div role="contentinfo">`   |
| `Search`        | `<search>`   | `<div role="search">`        |
| `Form`          | `<form>`     | `<div role="form">`          |
| `Region`        | `<section>`  | `<div role="region">`        |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **Form** and **Region** landmarks are only recognized by assistive technology when they have an accessible name (`aria-label` or `aria-labelledby`). The core emits a debug diagnostic when neither `labelledby_id` nor `messages.label` provides a name for these roles.
- Multiple `Navigation` or `Complementary` landmarks on the same page should each have a distinct `aria-label` to differentiate them (e.g., "Primary navigation", "Footer navigation").
- `Banner` and `ContentInfo` are page-level landmarks. Nesting them inside `<article>` or `<section>` changes their semantics — `Landmark` does not prevent this, but documentation warns against it.

> **Implicit landmark context:** `<header>` and `<footer>` elements only map to `banner` and `contentinfo` landmark roles when they are NOT descendants of `<article>`, `<aside>`, `<main>`, `<nav>`, or `<section>`. When nested inside sectioning content, they have no corresponding landmark role. Adapters SHOULD emit a `cfg(debug_assertions)` warning when using `Banner` or `ContentInfo` roles with `<header>`/`<footer>` elements, as the landmark semantics may be lost depending on DOM context.

Debug diagnostics are intentionally portable in the framework-agnostic core: use `log::warn!` under the workspace debug feature, fall back to `eprintln!` in native `std` debug builds when that feature is disabled, and leave browser-specific warning sinks to adapter crates.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the `Landmark` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible name for the landmark region (required for Form and Region roles).
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { label: MessageFn::static_str("") }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Library Parity

> Compared against: React Aria (`Landmark`).

### 5.1 Props

| Feature | ars-ui                             | React Aria                       | Notes                                |
| ------- | ---------------------------------- | -------------------------------- | ------------------------------------ |
| Role    | `role: Role`                       | via `useLandmark` role param     | Both libraries                       |
| Label   | `messages.label` / `labelledby_id` | `aria-label` / `aria-labelledby` | Both libraries                       |
| Locale  | `locale`                           | --                               | ars-ui addition for localized labels |

**Gaps:** None.

### 5.2 Anatomy

| Part | ars-ui                    | React Aria         | Notes                                         |
| ---- | ------------------------- | ------------------ | --------------------------------------------- |
| Root | `Root` (semantic element) | (semantic element) | Both libraries render semantic HTML5 elements |

**Gaps:** None.

### 5.3 Features

| Feature                     | ars-ui | React Aria |
| --------------------------- | ------ | ---------- |
| Semantic HTML5 elements     | Yes    | Yes        |
| Div fallback with ARIA role | Yes    | Yes        |
| All 8 landmark roles        | Yes    | Yes        |
| Form/Region name warning    | Yes    | Yes        |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria provides `useLandmark` hook; ars-ui provides a component with Props/Api pattern.
- **Recommended additions:** None.
