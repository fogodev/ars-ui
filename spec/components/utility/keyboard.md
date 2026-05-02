---
component: Keyboard
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: Keyboard
---

# Keyboard

A stateless utility component that renders keyboard shortcut text inside semantic `<kbd>` elements. Primarily used inside Menu item shortcut slots but applicable anywhere shortcut hints are shown.

## 1. API

### 1.1 Props

```rust
/// Props for the `Keyboard` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The keyboard shortcut text to display (e.g., "⌘C", "Ctrl+Shift+P").
    /// When platform_aware is true, modifier tokens are auto-mapped.
    pub shortcut: String,
    /// When true, automatically maps generic modifier tokens to platform-specific
    /// symbols: "Mod" → "⌘" (macOS) / "Ctrl" (others), "Alt" → "⌥" (macOS) / "Alt",
    /// "Shift" → "⇧" (macOS) / "Shift", "Meta" → "⌘" (macOS) / "Win" (others).
    /// Default: false (render shortcut text as-is).
    pub platform_aware: bool,
    /// Whether the current platform is macOS. The adapter provides this value
    /// via `PlatformEffects::is_mac_platform()` (see `01-architecture.md` §2.2.7).
    /// During SSR, defaults to `false` (non-Mac); client-side hydration updates
    /// the display if the actual platform differs.
    pub is_mac: bool,
    /// When true, the rendered `<kbd>` is purely decorative and the agnostic
    /// core emits `aria-hidden="true"` on the root so screen readers skip it.
    /// Use when the action is announced through some other path (e.g., the
    /// parent button's accessible name) and the `<kbd>` is a visual reminder
    /// only. Default: false. Inside Menu items, the parent `Shortcut` part
    /// already manages aria-hidden on its own wrapper, so leave this `false`
    /// there to avoid duplication.
    pub decorative: bool,
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "keyboard"]
pub enum Part {
    Root,
}

/// API for the `Keyboard` component.
///
/// Owns its `Props` (matching the stateless-Api convention shared with
/// `separator`, `visually_hidden`, `landmark`, etc. — see
/// `foundation/10-component-spec-template.md` §4.1.2).
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    pub const fn props(&self) -> &Props { &self.props }
    pub fn id(&self) -> &str { &self.props.id }
    pub fn shortcut(&self) -> &str { &self.props.shortcut }
    pub const fn platform_aware(&self) -> bool { self.props.platform_aware }
    pub const fn is_mac(&self) -> bool { self.props.is_mac }
    pub const fn decorative(&self) -> bool { self.props.decorative }

    /// Root `<kbd>` element attributes.
    ///
    /// Always emits `data-ars-scope="keyboard"` and `data-ars-part="root"`.
    /// When `decorative` is `true`, also emits `aria-hidden="true"` so
    /// assistive technology skips the purely-visual shortcut hint.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val).set(part_attr, part_val);
        if self.props.decorative {
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }
        attrs
    }

    /// Returns the formatted shortcut text, applying platform-aware
    /// modifier mapping when `platform_aware` is true.
    pub fn display_text(&self, locale: Option<&Locale>) -> String {
        if self.props.platform_aware {
            format_platform_shortcut(&self.props.shortcut, self.props.is_mac, locale)
        } else {
            self.props.shortcut.clone()
        }
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

// Platform detection is provided by the adapter via `PlatformEffects::is_mac_platform()`
// (see `01-architecture.md` §2.2.7). The `is_mac` parameter is passed into
// `format_platform_shortcut` by the caller. During SSR, the adapter defaults
// to `false` (non-Mac). Client-side hydration updates the display if the
// actual platform differs.

/// Modifier keys recognised by `format_platform_shortcut`.
///
/// Strongly typed so a typo in the formatter or in the localisation
/// table is a compile error rather than a silent fall-through to the
/// "unknown token" branch.
#[derive(Clone, Copy)]
enum Modifier { Mod, Shift, Alt, Ctrl, Meta }

impl Modifier {
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "Mod" => Self::Mod,
            "Shift" => Self::Shift,
            "Alt" => Self::Alt,
            "Ctrl" => Self::Ctrl,
            "Meta" => Self::Meta,
            _ => return None,
        })
    }

    const fn mac_symbol(self) -> &'static str {
        match self {
            // `Mod` and `Meta` both render as `⌘` on macOS.
            Self::Mod | Self::Meta => "⌘",
            Self::Shift => "⇧",
            Self::Alt => "⌥",
            Self::Ctrl => "⌃",
        }
    }

    const fn english_label(self) -> &'static str {
        match self {
            // `Mod` expands to `Ctrl` on non-mac platforms.
            Self::Mod | Self::Ctrl => "Ctrl",
            Self::Shift => "Shift",
            Self::Alt => "Alt",
            Self::Meta => "Win",
        }
    }
}

/// Returns the localised display name for a modifier key on non-macOS
/// platforms. Implements the §3.2 modifier-localisation table.
fn localized_modifier(modifier: Modifier, locale: Option<&Locale>) -> String {
    let lang = locale.map_or("en", Locale::language);
    match (modifier, lang) {
        (Modifier::Mod | Modifier::Ctrl, "de") => "Strg",
        (Modifier::Shift, "de") => "Umschalt",
        (Modifier::Shift, "fr") => "Maj",
        (Modifier::Shift, "es") => "Mayús",
        (Modifier::Shift, "it") => "Maiusc",
        _ => modifier.english_label(),
    }
    .to_string()
}

/// Maps generic modifier tokens to platform-specific symbols.
/// `is_mac` is provided by the adapter via `PlatformEffects::is_mac_platform()`.
fn format_platform_shortcut(shortcut: &str, is_mac: bool, locale: Option<&Locale>) -> String {
    shortcut
        .split('+')
        .map(|part| match (Modifier::parse(part), is_mac) {
            (Some(modifier), true)  => modifier.mac_symbol().to_string(),
            (Some(modifier), false) => localized_modifier(modifier, locale),
            (None, _)               => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(if is_mac { "" } else { "+" })
}
```

Adapter consumers may also construct `Props` through the chained-builder
form (`Props::new().shortcut("Mod+S").platform_aware(true).is_mac(true)`),
which mirrors the convention used by every other stateless utility in
`crates/ars-components/src/utility/`.

## 2. Anatomy

```text
Keyboard
└── Root  <kbd>  data-ars-scope="keyboard" data-ars-part="root"
```

| Part | Element | Key Attributes                                      |
| ---- | ------- | --------------------------------------------------- |
| Root | `<kbd>` | `data-ars-scope="keyboard"`, `data-ars-part="root"` |

**1 part total.** A single `<kbd>` element containing the formatted shortcut text.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- The `<kbd>` element is semantically appropriate for keyboard input text (HTML spec).
- When used inside Menu items, the parent `Shortcut` part already sets `aria-hidden="true"` on its wrapper — keep `Props::decorative = false` here so the agnostic core does not duplicate the attribute.
- When used standalone, the `<kbd>` text is announced by screen readers naturally. If the shortcut is purely decorative (e.g., the action is already announced through the parent button's accessible name), set `Props::decorative = true` and the agnostic core emits `aria-hidden="true"` directly on the `<kbd>`.
- Integration with `menu::Item.aria_keyshortcuts` for programmatic shortcut announcement is the consumer's responsibility (already defined in `selection/menu.md` §6).

### 3.2 Locale Handling

`KeyboardEvent.key` is already locale-aware — it returns the character for the user's keyboard layout (e.g., `"z"` on QWERTY, `"w"` on AZERTY for the same physical key). No additional `locale` prop is needed for key matching. The `platform_aware` prop handles modifier symbol mapping (⌘ vs Ctrl), which is platform-dependent, not locale-dependent.

When `locale` is provided, modifier key display names are localized:

| Modifier | English (default) | German (`de`) | French (`fr`) | Spanish (`es`) | Italian (`it`) | Japanese (`ja`) |
| -------- | ----------------- | ------------- | ------------- | -------------- | -------------- | --------------- |
| Ctrl     | Ctrl              | Strg          | Ctrl          | Ctrl           | Ctrl           | Ctrl            |
| Shift    | Shift             | Umschalt      | Maj           | Mayús          | Maiusc         | Shift           |
| Alt      | Alt               | Alt           | Alt           | Alt            | Alt            | Alt             |
| Meta     | ⌘ (macOS) / Win   | ⌘ / Win       | ⌘ / Win       | ⌘ / Win        | ⌘ / Win        | ⌘ / Win         |

When `locale` is `None`, English names are used. The table above covers all languages that localize modifier key labels on physical keyboards. All other languages (Chinese, Korean, Arabic, Russian, Portuguese, Hindi, Turkish, etc.) use English labels on their physical keyboards and require no localization. macOS uses universal symbols (⌘ ⇧ ⌥ ⌃) regardless of language.

## 4. Usage Patterns

**In a Menu item (composing with existing Shortcut part):**

```rust
// Leptos
view! {
    <div {..api.item_attrs(&key)}>
        <span {..api.item_text_attrs(&key)}>{item.label}</span>
        <kbd {..api.item_shortcut_attrs(&key)}>
            <Keyboard shortcut="Mod+C" platform_aware=true />
        </kbd>
    </div>
}
```

**Standalone shortcut hint:**

```rust
view! {
    <div class="shortcut-hint">
        <span>"Save"</span>
        <Keyboard shortcut="Mod+S" platform_aware=true />
    </div>
}
```

**Decorative shortcut hint** (the action is already announced through the
parent button's accessible name; the `<kbd>` is a visual reminder only):

```rust
view! {
    <button aria-label="Save document">
        "Save"
        <Keyboard shortcut="Mod+S" platform_aware=true decorative=true />
    </button>
}
```

## 5. Relationship to `Menu` `Shortcut`

The `Menu` component's `Shortcut` part (`selection/menu.md` §6) handles the **layout and accessibility** of shortcut hints within menu items. The `Keyboard` component handles the **rendering and formatting** of the shortcut text itself. They compose: `Keyboard` renders inside the `Menu`'s `Shortcut` slot.

## 6. Library Parity

> Compared against: React Aria (`Keyboard`).

### 6.1 Props

| Feature        | ars-ui           | React Aria | Notes                                                                           |
| -------------- | ---------------- | ---------- | ------------------------------------------------------------------------------- |
| Shortcut text  | `shortcut`       | children   | RA uses children; ars-ui uses a string prop                                     |
| Platform-aware | `platform_aware` | --         | ars-ui addition for auto-mapping modifier keys                                  |
| Decorative     | `decorative`     | --         | ars-ui addition: emits `aria-hidden="true"` on the root for purely-visual hints |

**Gaps:** None.

### 6.2 Anatomy

| Part | ars-ui           | React Aria           | Notes          |
| ---- | ---------------- | -------------------- | -------------- |
| Root | `Root` (`<kbd>`) | `Keyboard` (`<kbd>`) | Both libraries |

**Gaps:** None.

### 6.3 Features

| Feature                     | ars-ui | React Aria |
| --------------------------- | ------ | ---------- |
| Semantic `<kbd>` elements   | Yes    | Yes        |
| Platform modifier mapping   | Yes    | --         |
| Locale-aware modifier names | Yes    | --         |

**Gaps:** None.

### 6.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui adds platform-aware modifier mapping (`Mod` → platform symbol), locale-aware modifier names (de/fr/es/it), and the `decorative` prop. React Aria's Keyboard is a simpler `<kbd>` renderer with no platform or locale awareness. Both render the formatted shortcut as a single string inside one `<kbd>` rather than splitting into nested per-key elements.
- **Recommended additions:** None.
