---
component: ArsProvider
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: Environment
---

# ArsProvider

`ArsProvider` is the **single root provider** for the ars-ui library. It supplies shared configuration, platform capabilities, provider-scoped modality state, i18n resources, and style strategy to all descendant components. It MUST be rendered at (or near) the application root.

`ArsProvider` subsumes the formerly separate `LocaleProvider`, `PlatformEffectsProvider`, `IcuProvider`, `I18nProvider`, and `ArsStyleProvider`.

## 1. API

### 1.1 Props

```rust
/// Props for the `ArsProvider` component.
#[derive(Clone, Debug)]
pub struct Props {
    /// The active locale for i18n message formatting and text direction inference.
    /// Defaults to `Locale::parse("en-US").expect("en-US is always valid")`.
    pub locale: Option<Locale>,

    /// Explicit reading direction override. When `None`, direction is inferred
    /// from `locale`. See `04-internationalization.md` §3.1 for `Direction`.
    pub direction: Option<Direction>,

    /// Active color mode for theme-aware rendering. Defaults to `ColorMode::System`.
    pub color_mode: Option<ColorMode>,

    /// When `true`, all descendant interactive components render as disabled.
    /// Defaults to `false`.
    pub disabled: Option<bool>,

    /// When `true`, all descendant form fields render as read-only.
    /// Defaults to `false`.
    pub read_only: Option<bool>,

    /// Optional prefix prepended to all generated IDs (for micro-frontend isolation).
    pub id_prefix: Option<String>,

    /// ID of the container element for portal mounts.
    /// `None` means the platform default (e.g., `document.body` on web).
    pub portal_container_id: Option<String>,

    /// ID of the root node for focus scope and portal queries.
    /// `None` means the platform default.
    pub root_node_id: Option<String>,

    /// Platform capabilities for side effects (focus, timers, scroll-lock, positioning).
    /// Adapters provide platform-specific implementations (e.g., `WebPlatformEffects`).
    /// Defaults to `NullPlatformEffects` (no-op) for tests and SSR.
    pub platform: Option<Arc<dyn PlatformEffects>>,

    /// Shared input-modality state for this provider root.
    /// Defaults to `DefaultModalityContext`.
    pub modality: Option<Arc<dyn ModalityContext>>,

    /// Calendar/locale data provider for date-time components.
    /// Production uses `Icu4xProvider`; tests use `StubIcuProvider`.
    /// Defaults to `StubIcuProvider` (English-only).
    pub icu_provider: Option<Arc<dyn IcuProvider>>,

    /// Per-component translation message registries.
    /// Defaults to empty (components use built-in English defaults).
    pub i18n_registries: Option<Rc<I18nRegistries>>,

    /// CSS style injection strategy for all descendant ars components.
    /// Defaults to `StyleStrategy::Inline`.
    pub style_strategy: Option<StyleStrategy>,
}
```

### 1.2 Connect / API

`ArsProvider` is a context-only provider — it has no `Part` enum, no `ConnectApi`, and no `AttrMap` output. It publishes an `ArsContext` via the framework context system.

```rust
/// Provided via framework context (Leptos provide_context / Dioxus use_context_provider).
#[derive(Clone, Debug)]
pub struct ArsContext {
    locale: Locale,
    direction: Direction,
    color_mode: ColorMode,
    disabled: bool,
    read_only: bool,
    id_prefix: Option<String>,
    portal_container_id: Option<String>,
    root_node_id: Option<String>,
    platform: Arc<dyn PlatformEffects>,
    modality: Arc<dyn ModalityContext>,
    icu_provider: Arc<dyn IcuProvider>,
    i18n_registries: Arc<I18nRegistries>,
    style_strategy: StyleStrategy,
}

impl ArsContext {
    /// Returns the active locale.
    pub fn locale(&self) -> &Locale { &self.locale }
    /// Returns the reading direction (explicit or locale-inferred).
    pub fn direction(&self) -> Direction { self.direction }
    /// Returns the active color mode.
    pub fn color_mode(&self) -> ColorMode { self.color_mode }
    /// Returns `true` when all descendant components should be disabled.
    pub fn disabled(&self) -> bool { self.disabled }
    /// Returns `true` when all descendant form fields should be read-only.
    pub fn read_only(&self) -> bool { self.read_only }
    /// Returns the optional ID prefix for generated IDs.
    pub fn id_prefix(&self) -> Option<&str> { self.id_prefix.as_deref() }
    /// Returns the portal container element ID, if set.
    pub fn portal_container_id(&self) -> Option<&str> { self.portal_container_id.as_deref() }
    /// Returns the root node ID for focus/portal scoping, if set.
    pub fn root_node_id(&self) -> Option<&str> { self.root_node_id.as_deref() }
    /// Returns the platform capabilities trait object.
    pub fn platform(&self) -> Arc<dyn PlatformEffects> { Arc::clone(&self.platform) }
    /// Returns the provider-scoped modality context.
    pub fn modality(&self) -> Arc<dyn ModalityContext> { Arc::clone(&self.modality) }
    /// Returns the ICU calendar/locale data provider.
    pub fn icu_provider(&self) -> Arc<dyn IcuProvider> { Arc::clone(&self.icu_provider) }
    /// Returns the i18n translation registries.
    pub fn i18n_registries(&self) -> &I18nRegistries { &self.i18n_registries }
    /// Returns the active CSS style injection strategy.
    pub fn style_strategy(&self) -> &StyleStrategy { &self.style_strategy }
}

impl Default for ArsContext {
    fn default() -> Self {
        Self {
            locale: Locale::parse("en-US").expect("en-US is always valid"),
            direction: Direction::Ltr,
            color_mode: ColorMode::System,
            disabled: false,
            read_only: false,
            id_prefix: None,
            portal_container_id: None,
            root_node_id: None,
            platform: Arc::new(NullPlatformEffects),
            modality: Arc::new(DefaultModalityContext::new()),
            icu_provider: Arc::new(StubIcuProvider),
            i18n_registries: Rc::new(I18nRegistries::new()),
            style_strategy: StyleStrategy::Inline,
        }
    }
}
```

Consumers access the context via the framework's context API:

```rust
// Leptos
let ctx = use_context::<ArsContext>();

// Dioxus
let ctx = try_use_context::<ArsContext>();
```

Convenience hooks read from `ArsContext` with fallback defaults:

- `use_locale()` — locale, falls back to `en-US`
- `use_number_formatter()` — locale-aware number formatting derived from the active `ArsProvider` locale
- `use_platform_effects()` — platform capabilities
- `use_modality_context()` — provider-scoped input-modality state
- `use_icu_provider()` — calendar/locale data
- `use_style_strategy()` — CSS style strategy, falls back to `Inline`
- `resolve_messages::<M>()` — translation registries

> **Note:** `ArsContext` implements `Default` with sensible fallbacks: locale `en-US`, direction `Ltr`, color mode `System`, disabled `false`, read-only `false`, style strategy `Inline`.

## 2. Anatomy

```text
ArsProvider
└── Root <div>   dir="{direction}"
    └── {children}
```

`ArsProvider` renders a single `<div dir="{direction}">` wrapper to propagate reading direction to descendants. It is otherwise a logical wrapper — no additional DOM structure.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

`ArsProvider` has no ARIA semantics. It is invisible to assistive technology. The `dir` attribute on the wrapper propagates reading direction for correct BiDi layout.

## 4. Internal Usage

| Consumer                 | What it reads                                                                                      |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| `Portal`                 | `portal_container_id` — where to mount portal nodes                                                |
| `FocusScope`             | `root_node_id` — boundary for focus containment queries                                            |
| All i18n-dependent comps | `locale`, `direction` — for message formatting and layout                                          |
| ID generation (`use_id`) | `id_prefix` — prefix for micro-frontend isolation                                                  |
| Form field components    | `disabled`, `read_only` — cascade to descendant interactive components                             |
| Theme-aware components   | `color_mode` — for conditional rendering based on active color mode                                |
| All effect closures      | `platform` — via `use_platform_effects()` for focus, timers, scroll-lock, positioning, DOM queries |
| Focus / Hover / Press    | `modality` — via `use_modality_context()` for shared modality and global press state               |
| Date-time components     | `icu_provider` — via `use_icu_provider()` for calendar data (weekday names, month names, etc.)     |
| Numeric components       | `locale` — via `use_number_formatter()` for provider-derived number formatting                      |
| All stateful components  | `i18n_registries` — via `resolve_messages::<M>()` for per-component translation lookups            |
| All rendered components  | `style_strategy` — via `use_style_strategy()` for CSS injection method                             |

## 5. Library Parity

> Compared against: Ark UI (`Environment`).

Ark UI's `Environment` component provides `getRootNode()` for Shadow DOM and iframe scoping. ars-ui's `ArsProvider` serves a broader purpose — it is the single root provider covering locale, direction, color mode, accessibility cascades, ID prefixing, portal/focus scoping, platform capabilities, i18n, and style strategy. The concepts overlap only in portal/focus scoping.

| Feature                  | ars-ui                | Ark UI        | Notes                                    |
| ------------------------ | --------------------- | ------------- | ---------------------------------------- |
| Portal container scoping | `portal_container_id` | (getRootNode) | Both scope portal mounts                 |
| Focus scope root         | `root_node_id`        | (getRootNode) | Both scope focus containment             |
| Locale / direction       | Yes                   | --            | ars-ui addition                          |
| Color mode               | Yes                   | --            | ars-ui addition                          |
| Disabled / read-only     | Yes                   | --            | ars-ui addition                          |
| ID prefix                | Yes                   | --            | ars-ui addition                          |
| Platform effects         | Yes                   | --            | ars-ui addition                          |
| ICU / i18n registries    | Yes                   | --            | ars-ui addition                          |
| Style strategy (CSP)     | Yes                   | --            | ars-ui addition                          |
| Context-only provider    | Yes                   | Yes           | Both render no significant DOM structure |

**Gaps:** None. ars-ui is a superset.
