---
component: Group
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: [fieldset, action-group]
references:
    react-aria: Group
---

# Group

A general-purpose grouping component that propagates `disabled`, `invalid`, and `read_only` state to all descendant components via context. Renders as a `<div>` with `role="group"` (or `role="region"` / `role="presentation"`).

Unlike `Fieldset`, which renders `<fieldset>`/`<legend>` and is form-specific, Group is a lightweight wrapper for any set of related controls (e.g., a group of buttons sharing a disabled state, or a NumberField's input + increment/decrement button cluster).

**Ark UI equivalent:** — (no direct equivalent)
**React Aria equivalent:** Group

## 1. API

### 1.1 Props

```rust
use ars_i18n::Direction;

/// The ARIA role applied to the group container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GroupRole {
    /// `role="group"` — a set of related UI elements not important enough
    /// to be included in a page summary or table of contents.
    #[default]
    Group,
    /// `role="region"` — a landmark region that is significant enough to be
    /// listed in a page summary. Requires an accessible name.
    Region,
    /// `role="presentation"` — removes the grouping semantics. Children are
    /// still grouped visually but not semantically.
    Presentation,
}

/// Props for the `Group` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Whether the group and all contained controls are disabled.
    /// Propagated to children via `GroupContext`.
    pub disabled: bool,
    /// Whether the group is in an invalid state.
    /// Propagated to children via `GroupContext`.
    pub invalid: bool,
    /// Whether the group is read-only.
    /// Propagated to children via `GroupContext`.
    pub read_only: bool,
    /// The ARIA role for the group container. Default: `Group`.
    pub role: GroupRole,
    /// Layout direction for RTL support.
    pub dir: Option<Direction>,
}
```

`Props` also provides the workspace-standard builder chain (`Props::new()`,
`.id(...)`, `.disabled(true)`, `.invalid(true)`, `.read_only(true)`, `.role(...)`,
`.dir(...)`) matching every other stateless utility component.

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "group"]
pub enum Part {
    Root,
}

/// Context propagated to descendant components.
/// Children read this to inherit disabled/invalid/read_only state.
///
/// `Default` is derived so child components can fall back to a zero-state
/// context via `.unwrap_or_default()` when no parent `Group` is present.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupContext {
    /// Whether the group is disabled.
    pub disabled: bool,
    /// Whether the group is in an invalid state.
    pub invalid: bool,
    /// Whether the group is read-only.
    pub read_only: bool,
}

/// The API for the `Group` component. Owns its `Props` so adapters can
/// construct it once per render without tracking a separate borrow.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns the `GroupContext` to provide to descendants.
    pub const fn group_context(&self) -> GroupContext {
        GroupContext {
            disabled: self.props.disabled,
            invalid: self.props.invalid,
            read_only: self.props.read_only,
        }
    }

    /// Root element attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs
            .set(HtmlAttr::Id, &self.props.id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        // Role
        let role_str = match self.props.role {
            GroupRole::Group => "group",
            GroupRole::Region => "region",
            GroupRole::Presentation => "presentation",
        };
        attrs.set(HtmlAttr::Role, role_str);

        // State attributes — emitted unconditionally regardless of role
        // (see §3.1 for the accessibility rationale).
        if self.props.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.props.invalid {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Invalid), "true")
                .set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        if self.props.read_only {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true")
                .set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if let Some(dir) = self.props.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
        }

        attrs
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
Group
├── Root        <div>  data-ars-scope="group" data-ars-part="root" role="group"
└── {children}         Descendant components inherit state via GroupContext
```

| Part | Element | Key Attributes                                                                            |
| ---- | ------- | ----------------------------------------------------------------------------------------- |
| Root | `<div>` | `data-ars-scope="group"`, `data-ars-part="root"`, `role`, `aria-disabled`, `aria-invalid` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute          | Element | Source            | Notes                                                        |
| ------------------ | ------- | ----------------- | ------------------------------------------------------------ |
| `role`             | Root    | `props.role`      | `"group"` (default), `"region"`, or `"presentation"`         |
| `aria-disabled`    | Root    | `props.disabled`  | `"true"` when the group is disabled                          |
| `aria-invalid`     | Root    | `props.invalid`   | `"true"` when the group is in an invalid state               |
| `aria-readonly`    | Root    | `props.read_only` | `"true"` when the group is read-only                         |
| `aria-label`       | Root    | Consumer-provided | Accessible name for the group (required for `role="region"`) |
| `aria-labelledby`  | Root    | Consumer-provided | Alternative accessible name via referenced element           |
| `aria-describedby` | Root    | Consumer-provided | Links to description or error message                        |

- When `role` is `Region`, an accessible name (`aria-label` or `aria-labelledby`) is **required** per WAI-ARIA.
- State attributes (`aria-disabled`, `aria-invalid`, `aria-readonly`) are emitted on the root whenever the corresponding prop is `true`, **regardless of `role`**. WAI-ARIA 1.2 §5.4 explicitly preserves global states and properties on elements with `role="presentation"`; suppressing them there would only mask the visual state from assistive technology without changing the semantics. React Aria (parity reference, §5.1) also emits state attributes unconditionally.
- `aria-disabled` on a `role="group"` container does NOT natively propagate to children (unlike `<fieldset disabled>`). The adapter MUST use `GroupContext` to disable children programmatically.

### 3.2 Context Propagation

Adapters MUST provide `GroupContext` via the framework context system (`provide_context` in Leptos, `use_context_provider` in Dioxus). Child components that support disabling (Button, TextField, etc.) SHOULD read from `GroupContext` and merge with their own props:

```rust,no_check
// Inside a child component's adapter:
let group_ctx = use_context::<GroupContext>();
let effective_disabled = props.disabled || group_ctx.map_or(false, |g| g.disabled);
let effective_invalid = props.invalid || group_ctx.map_or(false, |g| g.invalid);
let effective_read_only = props.read_only || group_ctx.map_or(false, |g| g.read_only);
```

A component's own props always take precedence — if a component explicitly sets `disabled=false`, it is NOT overridden by the group's `disabled=true`. The merge uses logical OR: disabled if either the component or the group is disabled.

## 4. Internationalization

- In RTL layouts, the `dir` prop is forwarded to the root element. Child components inherit direction from the DOM cascade.
- Group has no text content and requires no localization strings.

## 5. Library Parity

> Compared against: React Aria (`Group`).

### 5.1 Props

| Feature   | ars-ui            | React Aria   | Notes                                            |
| --------- | ----------------- | ------------ | ------------------------------------------------ |
| Disabled  | `disabled`        | `isDisabled` | Both libraries                                   |
| Invalid   | `invalid`         | `isInvalid`  | Both libraries                                   |
| Read-only | `read_only`       | `isReadOnly` | Both libraries                                   |
| Role      | `role: GroupRole` | `role`       | Both libraries; ars-ui uses enum, RA uses string |
| Dir       | `dir`             | --           | ars-ui addition                                  |

**Gaps:** None.

### 5.2 Anatomy

| Part | ars-ui | React Aria | Notes                       |
| ---- | ------ | ---------- | --------------------------- |
| Root | `Root` | `Group`    | Both libraries; single-part |

**Gaps:** None.

### 5.3 Events

| Callback     | ars-ui | React Aria                | Notes                      |
| ------------ | ------ | ------------------------- | -------------------------- |
| Hover events | --     | `onHoverStart/End/Change` | RA exposes hover callbacks |

**Gaps:** None. Hover events for a grouping container are low-priority and not adopted.

### 5.4 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria exposes hover callbacks on Group; ars-ui omits them since Group is a state-propagation wrapper, not an interactive element.
- **Recommended additions:** None.
