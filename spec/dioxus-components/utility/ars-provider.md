---
adapter: dioxus
component: ars-provider
category: utility
source: components/utility/ars-provider.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ArsProvider — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ArsProvider`](../../components/utility/ars-provider.md) context contract to Dioxus 0.7.x. `ArsProvider` is the single root provider — it subsumes the formerly separate `LocaleProvider`, `ArsStyleProvider`, and `PlatformProvider`.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ArsProviderProps {
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,
    #[props(optional, into)]
    pub direction: Option<Signal<Direction>>,
    #[props(optional, into)]
    pub color_mode: Option<Signal<ColorMode>>,
    #[props(optional, into)]
    pub disabled: Option<Signal<bool>>,
    #[props(optional, into)]
    pub read_only: Option<Signal<bool>>,
    #[props(optional)]
    pub id_prefix: Option<String>,
    #[props(optional)]
    pub portal_container_id: Option<String>,
    #[props(optional)]
    pub root_node_id: Option<String>,
    #[props(optional)]
    pub platform: Option<ArsRc<dyn PlatformEffects>>,
    #[props(optional)]
    pub icu_provider: Option<Arc<dyn IcuProvider>>,
    #[props(optional)]
    pub i18n_registries: Option<Rc<I18nRegistries>>,
    #[props(optional)]
    pub style_strategy: Option<StyleStrategy>,
    #[props(optional)]
    pub dioxus_platform: Option<Rc<dyn DioxusPlatform>>,
    pub children: Element,
}

#[component]
pub fn ArsProvider(props: ArsProviderProps) -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core context props. Reactive props (`locale`, `direction`, `color_mode`, `disabled`, `read_only`) accept `Signal` for live updates. Non-reactive props (`id_prefix`, `portal_container_id`, `root_node_id`) are plain values set at mount time.
- `style_strategy` maps directly to the core `StyleStrategy` prop (defaults to `StyleStrategy::Inline`).
- `dioxus_platform` is an adapter-only extra prop (no core equivalent) — it provides Dioxus-specific platform services (file pickers, clipboard, drag data).
- Context parity: publishes full `ArsContext` to descendants via `use_context_provider`.

## 4. Part Mapping

| Core part / structure | Required?     | Adapter rendering target | Ownership      | Attr source | Notes                                 |
| --------------------- | ------------- | ------------------------ | -------------- | ----------- | ------------------------------------- |
| ars provider boundary | provider-only | `<div dir="{dir}">`      | adapter-owned  | direction   | Wrapper propagates `dir` attribute.   |
| children subtree      | required      | consumer children        | consumer-owned | none        | Descendants consume provided context. |

## 5. Attr Merge and Ownership Rules

| Target node     | Core attrs | Adapter-owned attrs            | Consumer attrs | Merge order    | Ownership notes                                |
| --------------- | ---------- | ------------------------------ | -------------- | -------------- | ---------------------------------------------- |
| `<div>` wrapper | none       | `dir` (derived from direction) | none           | not applicable | Minimal wrapper for BiDi direction propagation |

## 6. Composition / Context Contract

The adapter publishes `ArsContext` with `use_context_provider`. Consumers use `try_use_context::<ArsContext>()`. The convenience hooks `use_locale()` and `use_direction()` read from this context with fallback defaults. During Dioxus SSR, all context values are platform-agnostic and require no browser globals.

## 7. Prop Sync and Event Mapping

ArsProvider is context-driven rather than event-driven.

| Adapter prop                                         | Mode       | Sync trigger            | Update path                       | Visible effect                               | Notes                                       |
| ---------------------------------------------------- | ---------- | ----------------------- | --------------------------------- | -------------------------------------------- | ------------------------------------------- |
| `locale`                                             | controlled | signal change           | update ArsContext.locale          | descendants see updated locale               | direction auto-inferred when direction=None |
| `direction`                                          | controlled | signal change           | update ArsContext.direction       | `dir` attribute updates, layout reflows      | overrides locale-inferred direction         |
| `color_mode`                                         | controlled | signal change           | update ArsContext.color_mode      | theme-aware descendants re-render            |                                             |
| `disabled` / `read_only`                             | controlled | signal change           | update ArsContext fields          | descendant components enable/disable         |                                             |
| `id_prefix` / `portal_container_id` / `root_node_id` | controlled | prop change after mount | update provided context           | descendants resolve new ID prefix/targets    | non-reactive; set once at mount             |
| `style_strategy`                                     | controlled | prop set at mount       | update ArsContext.style_strategy  | descendant components use strategy for CSS   | defaults to `StyleStrategy::Inline`         |
| `dioxus_platform`                                    | controlled | prop set at mount       | update ArsContext.dioxus_platform | adapter-specific platform services available | defaults via feature flag resolution        |

## 8. Registration and Cleanup Contract

- No DOM registration lifecycle beyond the `<div>` wrapper.
- Cleanup is the lifetime of the provided context value only.
- Descendants must stop reading provider data after the provider unmounts.

| Registered entity   | Registration trigger | Identity key      | Cleanup trigger  | Cleanup action     | Notes                                            |
| ------------------- | -------------------- | ----------------- | ---------------- | ------------------ | ------------------------------------------------ |
| provided ArsContext | provider mount       | provider instance | provider cleanup | drop context value | no DOM listener ownership in the provider itself |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner      | Node availability | Composition rule | Notes                                             |
| ------------------ | ------------- | -------------- | ----------------- | ---------------- | ------------------------------------------------- |
| `<div>` wrapper    | no            | not applicable | server-safe       | not applicable   | Renders a minimal `dir` wrapper on all platforms. |

## 10. State Machine Boundary Rules

- machine-owned state: provided context values exposed through `ArsContext`.
- adapter-local derived bookkeeping: `Memo<Direction>` derived from locale signal when direction prop is `None`.
- forbidden local mirrors: do not keep separate unsynchronized copies outside the published context.
- allowed snapshot-read contexts: provider render/effects only.

## 11. Callback Payload Contract

| Callback           | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                      |
| ------------------ | -------------- | ------------- | -------------- | ----------- | ---------------------------------------------------------- |
| no public callback | none           | none          | not applicable | no          | Descendants consume context rather than callback payloads. |

## 12. Failure and Degradation Rules

| Condition                                                   | Policy             | Notes                                                                                  |
| ----------------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------- |
| locale signal absent (None prop)                            | use default        | Default to `en-US`, direction inferred as `Ltr`.                                       |
| descendant expects required environment data that is absent | warn and ignore    | The provider remains valid; required-descendant behavior belongs to the consumer spec. |
| platform unavailable during SSR or server-only rendering    | degrade gracefully | All context values are platform-agnostic; no platform lookup needed.                   |
| `style_strategy` absent                                     | use default        | Default to `StyleStrategy::Inline`.                                                    |
| `dioxus_platform` absent                                    | use default        | Resolved via feature flags: Web > Desktop > NullPlatform.                              |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                        | Notes                                       |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | -------------------------------------------------------------- | ------------------------------------------- |
| provided ArsContext              | instance-derived | not applicable      | not applicable                           | provider instance identity must remain stable across hydration | Provider lifetime is the identity boundary. |

## 14. SSR and Client Boundary Rules

- The provider renders a `<div dir>` wrapper during SSR with the initial direction value.
- All context values are platform-agnostic (no browser dependencies in the context itself).
- Platform-specific operations go through `PlatformEffects` or `DioxusPlatform`, not through ArsProvider.
- On server rendering, no platform lookup is required — ArsProvider publishes plain configuration values.

## 15. Performance Constraints

- Provided context should only update when incoming signal values actually change.
- The `direction` memo only recomputes when locale or direction prop changes.
- Chain `dir` attribute off the `direction` memo to avoid redundant re-reads of `locale`.

## 16. Implementation Dependencies

| Dependency    | Required?   | Dependency type     | Why it must exist first                                                             | Notes                                                                                       |
| ------------- | ----------- | ------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `dismissable` | recommended | downstream consumer | Dismissable-like utilities often consume environment context for scoped DOM access. | Dependency is about implementation order in the wider utility layer, not runtime ownership. |
| `focus-scope` | recommended | downstream consumer | Focus utilities may rely on scoped environment data in composite trees.             | Same guidance applies to future overlay utilities.                                          |

## 17. Recommended Implementation Sequence

1. Resolve all props to signals (defaulting absent signals to stored defaults).
2. Derive `direction` memo from locale when direction prop is `None`.
3. Resolve `style_strategy` (default `StyleStrategy::Inline`).
4. Resolve `dioxus_platform` via feature flag fallback (Web > Desktop > NullPlatform).
5. Build and publish `ArsContext` via `use_context_provider`.
6. Render `div { dir: "{dir}", {children} }`.
7. Verify descendant consumption and provider cleanup.

## 18. Anti-Patterns

- Do not fail hard when optional props are absent — use documented defaults.
- Do not add heavy DOM structure beyond the `<div dir>` wrapper.
- Do not attempt platform lookup (e.g., `use_platform()`) inside ArsProvider — it provides configuration, not platform capabilities.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `ArsContext` is always present when rendered inside an `ArsProvider`.
- Consumers may assume `use_locale()` returns a valid locale signal even without a provider (fallback to `en-US`).
- Consumers may assume the `dir` attribute on the wrapper reflects the current direction.

## 20. Platform Support Matrix

| Capability / behavior   | Web          | Desktop      | Mobile       | SSR          | Notes                                      |
| ----------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------------ |
| context publication     | full support | full support | full support | full support | All context values are platform-agnostic.  |
| `dir` wrapper rendering | full support | full support | full support | full support | Renders `<div dir>` on all Dioxus targets. |

## 21. Debug Diagnostics and Production Policy

| Condition                   | Debug build behavior | Production behavior | Notes                                    |
| --------------------------- | -------------------- | ------------------- | ---------------------------------------- |
| no ArsProvider in ancestors | debug warning        | degrade gracefully  | `use_locale()` returns fallback `en-US`. |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                  | Reused by                       | Notes                                   |
| ------------------- | --------- | ----------------------------------------------- | ------------------------------- | --------------------------------------- |
| `use_locale()` hook | required  | Read locale from ArsContext with en-US fallback | all locale-dependent components | Defined in `09-adapter-dioxus.md` §16.1 |

## 23. Framework-Specific Behavior

Dioxus targets web, desktop, and mobile. `ArsContext` bundles platform-agnostic configuration, the `PlatformEffects` trait object, and the Dioxus-specific `DioxusPlatform` trait object. The `dioxus_platform` prop is an adapter-only extra with no core equivalent — it provides Dioxus-specific platform services (file pickers, clipboard, drag data) not covered by `PlatformEffects`. The Dioxus adapter resolves the platform implementation via feature flags: `WebPlatform` (web), `DesktopPlatform` (desktop), `NullPlatform` (SSR/tests/mobile fallback). The `style_strategy` field controls CSS injection for all descendant ars components.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ArsProviderSketchProps {
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,
    #[props(optional, into)]
    pub direction: Option<Signal<Direction>>,
    #[props(optional, into)]
    pub color_mode: Option<Signal<ColorMode>>,
    #[props(optional, into)]
    pub disabled: Option<Signal<bool>>,
    #[props(optional, into)]
    pub read_only: Option<Signal<bool>>,
    #[props(optional)]
    pub id_prefix: Option<String>,
    #[props(optional)]
    pub portal_container_id: Option<String>,
    #[props(optional)]
    pub root_node_id: Option<String>,
    #[props(optional)]
    pub platform: Option<ArsRc<dyn PlatformEffects>>,
    #[props(optional)]
    pub icu_provider: Option<Arc<dyn IcuProvider>>,
    #[props(optional)]
    pub i18n_registries: Option<Rc<I18nRegistries>>,
    #[props(optional)]
    pub style_strategy: Option<StyleStrategy>,
    #[props(optional)]
    pub dioxus_platform: Option<Rc<dyn DioxusPlatform>>,
    pub children: Element,
}

#[component]
pub fn ArsProvider(props: ArsProviderSketchProps) -> Element {
    let locale = props.locale.unwrap_or_else(|| {
        use_signal(|| Locale::parse("en-US").expect("en-US is always valid"))
    });
    let direction = use_memo(move || {
        props.direction
            .map(|d| d())
            .unwrap_or_else(|| locale.read().direction())
    });
    let color_mode = props.color_mode.unwrap_or_else(|| use_signal(|| ColorMode::System));
    let disabled = props.disabled.unwrap_or_else(|| use_signal(|| false));
    let read_only = props.read_only.unwrap_or_else(|| use_signal(|| false));
    let platform = props.platform.unwrap_or_else(|| resolve_platform_for_target());
    let icu_provider = props.icu_provider.unwrap_or_else(|| Arc::new(StubIcuProvider));
    let i18n_registries = props.i18n_registries.unwrap_or_else(|| Rc::new(I18nRegistries::new()));
    let style_strategy = props.style_strategy.unwrap_or(StyleStrategy::Inline);
    let dioxus_platform = props.dioxus_platform.unwrap_or_else(|| {
        #[cfg(feature = "web")]
        { Rc::new(WebPlatform) }
        #[cfg(all(feature = "desktop", not(feature = "web")))]
        { Rc::new(DesktopPlatform) }
        #[cfg(not(any(feature = "web", feature = "desktop")))]
        { Rc::new(NullPlatform) }
    });

    use_context_provider(|| ArsContext {
        locale,
        direction,
        color_mode,
        disabled,
        read_only,
        id_prefix: use_signal(|| props.id_prefix),
        portal_container_id: use_signal(|| props.portal_container_id),
        root_node_id: use_signal(|| props.root_node_id),
        platform,
        icu_provider,
        i18n_registries,
        style_strategy,
        dioxus_platform,
    });

    let dir = use_memo(move || direction().as_html_attr().to_string());

    rsx! {
        div { dir: "{dir}", {props.children} }
    }
}
```

## 25. Adapter Invariants

- Context publication and consumption rules must remain explicit so descendants know when environment data is available.
- The `dir` attribute on the wrapper MUST reactively update when direction changes.
- `use_locale()` MUST always return a valid signal, even without a provider.
- Dioxus SSR MUST NOT require a live platform context — ArsProvider publishes configuration only.

## 26. Accessibility and SSR Notes

The `<div dir>` wrapper propagates reading direction for correct BiDi layout. No ARIA semantics. SSR renders the wrapper with initial direction value. No platform lookup is needed during SSR.

## 27. Parity Summary and Intentional Deviations

Parity summary: full core context parity.

Intentional deviations:

- `dioxus_platform` is an adapter-only extra prop with no core equivalent. It provides Dioxus-specific platform services via the `DioxusPlatform` trait (§6 in `09-adapter-dioxus.md`).

## 28. Test Scenarios

- context publication with all props
- default fallback values when props are absent
- `use_locale()` fallback without provider
- `dir` attribute reactivity on locale change
- SSR rendering on all Dioxus targets (web, desktop, mobile)
- `style_strategy` defaults to `StyleStrategy::Inline` when absent
- `dioxus_platform` resolves via feature flags when absent

## 29. Test Oracle Notes

| Behavior                | Preferred oracle type | Notes                                                                                |
| ----------------------- | --------------------- | ------------------------------------------------------------------------------------ |
| provider publication    | context registration  | Assert descendants observe the published ArsContext.                                 |
| locale fallback         | context registration  | Assert `use_locale()` returns en-US without a provider.                              |
| dir attribute           | DOM assertion         | Assert `dir` attribute matches current direction.                                    |
| SSR platform fallback   | context registration  | Verify context is published without platform lookup during SSR.                      |
| style strategy default  | context registration  | Assert `use_style_strategy()` returns `Inline` without explicit prop.                |
| dioxus platform default | context registration  | Assert `use_platform()` returns feature-flag-appropriate impl without explicit prop. |

## 30. Implementation Checklist

- [ ] Provider context publishes all documented `ArsContext` fields.
- [ ] `<div dir>` wrapper renders and reactively updates.
- [ ] Default fallback values match documented defaults (en-US, Ltr, System, false, false, Inline).
- [ ] `use_locale()` works with and without provider.
- [ ] SSR renders without platform lookup.
- [ ] `style_strategy` defaults to `StyleStrategy::Inline`.
- [ ] `dioxus_platform` defaults via feature flag resolution (Web > Desktop > NullPlatform).
- [ ] Context-registration test oracles are covered.
