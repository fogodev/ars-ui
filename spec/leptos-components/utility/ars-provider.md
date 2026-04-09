---
adapter: leptos
component: ars-provider
category: utility
source: components/utility/ars-provider.md
source_foundation: foundation/08-adapter-leptos.md
---

# ArsProvider — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ArsProvider`](../../components/utility/ars-provider.md) context contract to Leptos 0.8.x. `ArsProvider` is the single root provider — it subsumes the formerly separate `LocaleProvider`, `PlatformEffectsProvider`, `IcuProvider`, `I18nProvider`, and `ArsStyleProvider`.

## 2. Public Adapter API

```rust
#[component]
pub fn ArsProvider(
    #[prop(optional, into)] locale: Option<Signal<Locale>>,
    #[prop(optional, into)] direction: Option<Signal<Direction>>,
    #[prop(optional, into)] color_mode: Option<Signal<ColorMode>>,
    #[prop(optional, into)] disabled: Option<Signal<bool>>,
    #[prop(optional, into)] read_only: Option<Signal<bool>>,
    #[prop(optional)] id_prefix: Option<String>,
    #[prop(optional)] portal_container_id: Option<String>,
    #[prop(optional)] root_node_id: Option<String>,
    #[prop(optional)] platform: Option<ArsRc<dyn PlatformEffects>>,
    #[prop(optional)] icu_provider: Option<ArsRc<dyn IcuProvider>>,
    #[prop(optional)] i18n_registries: Option<ArsRc<I18nRegistries>>,
    #[prop(optional)] style_strategy: Option<StyleStrategy>,
    children: Children,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core context props. Reactive props (`locale`, `direction`, `color_mode`, `disabled`, `read_only`) accept `Signal` for live updates. Non-reactive props (`id_prefix`, `portal_container_id`, `root_node_id`, `style_strategy`) are plain values set at mount time.
- Context parity: publishes full `ArsContext` to descendants via `provide_context`.

## 4. Part Mapping

| Core part / structure | Required?     | Adapter rendering target | Ownership      | Attr source | Notes                                 |
| --------------------- | ------------- | ------------------------ | -------------- | ----------- | ------------------------------------- |
| ars provider boundary | provider-only | `<div dir=dir_attr>`     | adapter-owned  | direction   | Wrapper propagates `dir` attribute.   |
| children subtree      | required      | consumer children        | consumer-owned | none        | Descendants consume provided context. |

## 5. Attr Merge and Ownership Rules

| Target node     | Core attrs | Adapter-owned attrs            | Consumer attrs | Merge order    | Ownership notes                                |
| --------------- | ---------- | ------------------------------ | -------------- | -------------- | ---------------------------------------------- |
| `<div>` wrapper | none       | `dir` (derived from direction) | none           | not applicable | Minimal wrapper for BiDi direction propagation |

## 6. Composition / Context Contract

The adapter publishes `ArsContext` with `provide_context`. Consumers use `use_context::<ArsContext>()`. The convenience hooks `use_locale()` and `use_direction()` read from this context with fallback defaults.

## 7. Prop Sync and Event Mapping

ArsProvider is context-driven rather than event-driven.

| Adapter prop                                         | Mode       | Sync trigger            | Update path                  | Visible effect                            | Notes                                             |
| ---------------------------------------------------- | ---------- | ----------------------- | ---------------------------- | ----------------------------------------- | ------------------------------------------------- |
| `locale`                                             | controlled | signal change           | update ArsContext.locale     | descendants see updated locale            | direction auto-inferred when direction=None       |
| `direction`                                          | controlled | signal change           | update ArsContext.direction  | `dir` attribute updates, layout reflows   | overrides locale-inferred direction               |
| `color_mode`                                         | controlled | signal change           | update ArsContext.color_mode | theme-aware descendants re-render         |                                                   |
| `disabled` / `read_only`                             | controlled | signal change           | update ArsContext fields     | descendant components enable/disable      |                                                   |
| `id_prefix` / `portal_container_id` / `root_node_id` | controlled | prop change after mount | update provided context      | descendants resolve new ID prefix/targets | non-reactive; set once at mount                   |
| `icu_provider`                                       | controlled | prop at mount           | stored in ArsContext         | date-time components use ICU data         | non-reactive; set once at mount                   |
| `i18n_registries`                                    | controlled | prop at mount           | stored in ArsContext         | components resolve translated messages    | non-reactive; set once at mount                   |
| `style_strategy`                                     | controlled | prop at mount           | stored in ArsContext         | components use strategy for CSS injection | non-reactive; defaults to `StyleStrategy::Inline` |

## 8. Registration and Cleanup Contract

- No DOM registration lifecycle beyond the `<div>` wrapper.
- Cleanup is the lifetime of the provided context value only.
- Descendants must stop reading provider data after the provider unmounts.

| Registered entity   | Registration trigger | Identity key      | Cleanup trigger  | Cleanup action     | Notes                                            |
| ------------------- | -------------------- | ----------------- | ---------------- | ------------------ | ------------------------------------------------ |
| provided ArsContext | provider mount       | provider instance | provider cleanup | drop context value | no DOM listener ownership in the provider itself |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner      | Node availability | Composition rule | Notes                                       |
| ------------------ | ------------- | -------------- | ----------------- | ---------------- | ------------------------------------------- |
| `<div>` wrapper    | no            | not applicable | server-safe       | not applicable   | Renders a minimal `dir` wrapper on SSR too. |

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

| Condition                                                   | Policy          | Notes                                                                                  |
| ----------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------- |
| locale signal absent (None prop)                            | use default     | Default to `en-US`, direction inferred as `Ltr`.                                       |
| descendant expects required environment data that is absent | warn and ignore | The provider remains valid; required-descendant behavior belongs to the consumer spec. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                        | Notes                                       |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | -------------------------------------------------------------- | ------------------------------------------- |
| provided ArsContext              | instance-derived | not applicable      | not applicable                           | provider instance identity must remain stable across hydration | Provider lifetime is the identity boundary. |

## 14. SSR and Client Boundary Rules

- The provider renders a `<div dir>` wrapper during SSR with the initial direction value.
- All context values are SSR-safe (no browser dependencies in the context itself).
- Platform-specific operations go through `PlatformEffects`, not through ArsProvider.

## 15. Performance Constraints

- Provided context should only update when incoming signal values actually change.
- The `direction` memo only recomputes when locale or direction prop changes.
- Do not recreate context values unnecessarily on unrelated rerenders.

## 16. Implementation Dependencies

| Dependency    | Required?   | Dependency type     | Why it must exist first                                                             | Notes                                                                                       |
| ------------- | ----------- | ------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `dismissable` | recommended | downstream consumer | Dismissable-like utilities often consume environment context for scoped DOM access. | Dependency is about implementation order in the wider utility layer, not runtime ownership. |
| `focus-scope` | recommended | downstream consumer | Focus utilities may rely on scoped environment data in composite trees.             | Same guidance applies to future overlay utilities.                                          |

## 17. Recommended Implementation Sequence

1. Resolve all props to signals (defaulting absent signals to stored defaults).
2. Derive `direction` memo from locale when direction prop is `None`.
3. Build and publish `ArsContext` via `provide_context`.
4. Render `<div dir=dir_attr>{children()}</div>`.
5. Verify descendant consumption and provider cleanup.

## 18. Anti-Patterns

- Do not fail hard when optional props are absent — use documented defaults.
- Do not add heavy DOM structure beyond the `<div dir>` wrapper.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `ArsContext` is always present when rendered inside an `ArsProvider`.
- Consumers may assume `use_locale()` returns a valid locale signal even without a provider (fallback to `en-US`).
- Consumers may assume the `dir` attribute on the wrapper reflects the current direction.

## 20. Platform Support Matrix

| Capability / behavior   | Browser client | SSR          | Notes                                             |
| ----------------------- | -------------- | ------------ | ------------------------------------------------- |
| context publication     | full support   | full support | All context values are platform-agnostic.         |
| `dir` wrapper rendering | full support   | full support | Renders `<div dir>` in both client and SSR paths. |

## 21. Debug Diagnostics and Production Policy

| Condition                   | Debug build behavior | Production behavior | Notes                                    |
| --------------------------- | -------------------- | ------------------- | ---------------------------------------- |
| no ArsProvider in ancestors | debug warning        | degrade gracefully  | `use_locale()` returns fallback `en-US`. |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required? | Responsibility                                   | Reused by                                   | Notes                                   |
| --------------------------- | --------- | ------------------------------------------------ | ------------------------------------------- | --------------------------------------- |
| `use_locale()` hook         | required  | Read locale from ArsContext with en-US fallback  | all locale-dependent components             | Defined in `08-adapter-leptos.md` §13.1 |
| `use_direction()` hook      | required  | Read direction from ArsContext with Ltr fallback | all direction-dependent components          | Parallel to `use_locale()`              |
| `use_style_strategy()` hook | required  | Read style strategy from ArsContext              | all components using `attr_map_to_leptos()` | Defined in `08-adapter-leptos.md` §3.4  |

## 23. Framework-Specific Behavior

Leptos is web-only. All `ArsContext` values are platform-agnostic. Platform-specific operations (focus, scroll-lock, positioning) go through `PlatformEffects`, not ArsProvider.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ArsProvider(
    #[prop(optional, into)] locale: Option<Signal<Locale>>,
    #[prop(optional, into)] direction: Option<Signal<Direction>>,
    #[prop(optional, into)] color_mode: Option<Signal<ColorMode>>,
    #[prop(optional, into)] disabled: Option<Signal<bool>>,
    #[prop(optional, into)] read_only: Option<Signal<bool>>,
    #[prop(optional)] id_prefix: Option<String>,
    #[prop(optional)] portal_container_id: Option<String>,
    #[prop(optional)] root_node_id: Option<String>,
    #[prop(optional)] platform: Option<ArsRc<dyn PlatformEffects>>,
    #[prop(optional)] icu_provider: Option<ArsRc<dyn IcuProvider>>,
    #[prop(optional)] i18n_registries: Option<ArsRc<I18nRegistries>>,
    #[prop(optional)] style_strategy: Option<StyleStrategy>,
    children: Children,
) -> impl IntoView {
    let locale = locale.unwrap_or_else(|| {
        Signal::stored(Locale::parse("en-US").expect("en-US is always valid"))
    });
    let direction = Memo::new(move |_| {
        direction
            .map(|d| d.get())
            .unwrap_or_else(|| locale.get().direction())
    });
    let color_mode = color_mode.unwrap_or_else(|| Signal::stored(ColorMode::System));
    let disabled = disabled.unwrap_or_else(|| Signal::stored(false));
    let read_only = read_only.unwrap_or_else(|| Signal::stored(false));
    let platform = platform.unwrap_or_else(|| Rc::new(WebPlatformEffects));
    let icu_provider = icu_provider.unwrap_or_else(|| Arc::new(StubIcuProvider));
    let i18n_registries = i18n_registries.unwrap_or_else(|| Rc::new(I18nRegistries::new()));
    let style_strategy = style_strategy.unwrap_or(StyleStrategy::Inline);

    provide_context(ArsContext {
        locale,
        direction,
        color_mode,
        disabled,
        read_only,
        id_prefix: Signal::stored(id_prefix),
        portal_container_id: Signal::stored(portal_container_id),
        root_node_id: Signal::stored(root_node_id),
        platform,
        icu_provider,
        i18n_registries,
        style_strategy,
    });

    let dir_attr = move || direction.get().as_html_attr();

    view! {
        <div dir=dir_attr>
            {children()}
        </div>
    }
}
```

## 25. Adapter Invariants

- Context publication and consumption rules must remain explicit so descendants know when environment data is available.
- The `dir` attribute on the wrapper MUST reactively update when direction changes.
- `use_locale()` MUST always return a valid signal, even without a provider.

## 26. Accessibility and SSR Notes

The `<div dir>` wrapper propagates reading direction for correct BiDi layout. No ARIA semantics. SSR renders the wrapper with initial direction value.

## 27. Parity Summary and Intentional Deviations

Parity summary: full core context parity.

Intentional deviations: none.

## 28. Test Scenarios

- context publication with all props
- default fallback values when props are absent
- `use_locale()` fallback without provider
- `dir` attribute reactivity on locale change
- SSR rendering
- `use_style_strategy()` returns `StyleStrategy::Inline` by default
- `use_style_strategy()` returns configured strategy when explicitly set

## 29. Test Oracle Notes

| Behavior               | Preferred oracle type | Notes                                                                 |
| ---------------------- | --------------------- | --------------------------------------------------------------------- |
| provider publication   | context registration  | Assert descendants observe the published ArsContext.                  |
| locale fallback        | context registration  | Assert `use_locale()` returns en-US without a provider.               |
| dir attribute          | DOM assertion         | Assert `dir` attribute matches current direction.                     |
| style strategy default | context registration  | Assert `use_style_strategy()` returns `Inline` without explicit prop. |

## 30. Implementation Checklist

- [ ] Provider context publishes all documented `ArsContext` fields.
- [ ] `<div dir>` wrapper renders and reactively updates.
- [ ] Default fallback values match documented defaults (en-US, Ltr, System, false, false, Inline).
- [ ] `use_locale()` works with and without provider.
- [ ] `use_style_strategy()` returns `StyleStrategy::Inline` by default.
- [ ] SSR renders the wrapper with correct `dir`.
- [ ] Context-registration test oracles are covered.
