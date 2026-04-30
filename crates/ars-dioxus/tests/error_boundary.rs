//! Unit tests for `error_boundary::Boundary` (issue #197).
//!
//! Exercises `spec/components/utility/error-boundary.md` and the
//! Dioxus-specific contract at `spec/dioxus-components/utility/error-boundary.md`:
//!
//! - children render when no descendant errors
//! - the canonical fallback renders with `role="alert"`,
//!   `aria-live="assertive"`, `aria-atomic="true"`, `data-ars-error="true"`,
//!   `data-ars-error-count`, and the `<ul>`/`<li>` error list when a child
//!   returns an error
//! - the `on_error` telemetry hook fires for each captured error
//! - a custom `fallback` prop replaces the default markup entirely
//! - the boundary catches the error so siblings around the wrapper still
//!   render

#![cfg(not(target_arch = "wasm32"))]

use std::{cell::RefCell, sync::Arc};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, MessageFn, MessagesRegistry,
    NullPlatformEffects, StyleStrategy,
};
use ars_dioxus::{
    ArsContext, NullPlatform,
    error_boundary::{self, Boundary, default_fallback},
};
use ars_i18n::{Direction, Locale, StubIntlBackend};
use dioxus::{CapturedError, dioxus_core::NoOpMutations, prelude::*};

/// Renders `app` to an HTML string via `dioxus-ssr`.
///
/// After the initial `rebuild_in_place`, [`ErrorBoundary`] schedules its
/// fallback via `needs_update()` rather than substituting it during the
/// same render pass; we drain pending work with two `render_immediate`
/// calls (mirroring the upstream `dioxus-core` test harness in
/// `dioxus-core/tests/error_boundary.rs::clear_error_boundary`) so the
/// error fallback shows up in the SSR string.
fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    vdom.render_immediate(&mut NoOpMutations);
    vdom.render_immediate(&mut NoOpMutations);

    dioxus_ssr::render(&vdom)
}

/// A component whose render function unconditionally errors. Stand-in for
/// "a descendant machine panicked or returned `Err`" — Dioxus only routes
/// errors to the nearest `ErrorBoundary` when they originate inside a
/// component scope, not from inline rsx expressions in the parent.
#[component]
fn ThrowingChild() -> Element {
    Err(CapturedError::from_display("boom-from-child").into())
}

#[test]
fn renders_children_when_no_error() {
    fn app() -> Element {
        rsx! {
            Boundary {
                p { "child-ok" }
            }
        }
    }

    let html = render_app(app);

    assert!(html.contains("child-ok"), "expected child rendered: {html}");
    assert!(
        !html.contains("data-ars-error"),
        "fallback should not render on the happy path: {html}"
    );
}

#[test]
fn renders_canonical_fallback_attrs_when_child_errors() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-scope="error-boundary""#),
        "missing scope attr: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="root""#),
        "missing root part attr: {html}"
    );
    assert!(
        html.contains(r#"data-ars-error="true""#),
        "missing data-ars-error: {html}"
    );
    assert!(
        html.contains(r#"data-ars-error-count="1""#),
        "missing data-ars-error-count: {html}"
    );
    assert!(
        html.contains(r#"role="alert""#),
        "missing role=alert: {html}"
    );
    assert!(
        html.contains(r#"aria-live="assertive""#),
        "missing aria-live: {html}"
    );
    assert!(
        html.contains(r#"aria-atomic="true""#),
        "missing aria-atomic: {html}"
    );
}

#[test]
fn fallback_includes_default_message_and_error_text() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("A component encountered an error."),
        "missing static message: {html}"
    );
    // The error text is wrapped in `<li>` (canonical multi-error shape).
    assert!(
        html.contains(r#"data-ars-part="item""#),
        "missing item part: {html}"
    );
    assert!(
        html.contains("boom-from-child"),
        "missing error text: {html}"
    );
}

#[test]
fn fallback_wraps_errors_in_unordered_list() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-part="list""#),
        "missing list part: {html}"
    );
    assert!(html.contains("<ul"), "expected <ul>: {html}");
    assert!(html.contains("<li"), "expected <li>: {html}");
}

#[test]
fn boundary_does_not_propagate_error_past_wrapper() {
    fn app() -> Element {
        rsx! {
            div { "data-outside": "1",
                Boundary { ThrowingChild {} }
                p { "after-boundary" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("after-boundary"),
        "sibling did not render: {html}"
    );
    assert!(
        html.contains(r#"data-outside="1""#),
        "outside marker missing: {html}"
    );
}

#[test]
fn on_error_telemetry_fires_with_captured_error() {
    // `on_error` is invoked from inside the `handle_error` closure during
    // render, so we capture into a thread-local-style cell shared with the
    // app fixture.
    thread_local! {
        static CAPTURED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn app() -> Element {
        let on_error = use_callback(|err: CapturedError| {
            CAPTURED.with(|c| c.borrow_mut().push(err.to_string()));
        });

        rsx! {
            Boundary { on_error: EventHandler::new(move |err: CapturedError| on_error.call(err)),
                ThrowingChild {}
            }
        }
    }

    CAPTURED.with(|c| c.borrow_mut().clear());

    let _html = render_app(app);

    let captured = CAPTURED.with(|c| c.borrow().clone());

    assert!(
        captured.iter().any(|m| m.contains("boom-from-child")),
        "on_error did not fire with the captured error; got {captured:?}"
    );
}

#[test]
fn custom_fallback_replaces_default_markup() {
    fn app() -> Element {
        let fallback = use_callback(|_ctx: ErrorContext| {
            rsx! {
                div { class: "my-custom-error", "Custom UI" }
            }
        });

        rsx! {
            Boundary { fallback, ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("Custom UI"),
        "custom fallback did not render: {html}"
    );
    assert!(
        !html.contains("data-ars-error"),
        "default markup should not render when fallback override is provided: {html}"
    );
}

#[test]
fn default_fallback_helper_emits_canonical_markup() {
    // `default_fallback` is the function consumers pass directly to a raw
    // `dioxus_core::ErrorBoundary` when they do not want the wrapper.
    fn app() -> Element {
        rsx! {
            ErrorBoundary { handle_error: default_fallback, ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(html.contains(r#"data-ars-error="true""#));
    assert!(html.contains("A component encountered an error."));
}

#[test]
fn custom_messages_override_default_heading() {
    fn app() -> Element {
        let messages = error_boundary::Messages {
            message: MessageFn::static_str("Algo deu errado."),
        };

        rsx! {
            Boundary { messages, ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("Algo deu errado."),
        "messages override did not render: {html}"
    );
    assert!(
        !html.contains("A component encountered an error."),
        "default English string should not render when override is provided: {html}"
    );
}

#[test]
fn happy_path_emits_zero_count_after_clear() {
    // Sanity: error_count = 0 only on the no-error path; the fallback never
    // emits zero in normal use, but we exercise the API directly here to keep
    // the count contract explicit.
    let api = error_boundary::Api::new(0);

    let attrs = api.root_attrs();

    let count = attrs.get(&ars_core::HtmlAttr::Data("ars-error-count"));

    assert_eq!(count, Some("0"));
}

/// Verifies that the canonical fallback markup is invariant under
/// RTL locales. The component does not branch on text direction
/// (the `data-ars-error*` attrs and `role="alert"` are
/// direction-agnostic per WAI-ARIA), so an RTL locale must produce
/// the same attribute set as an LTR locale. Catches a regression
/// where the boundary accidentally consumes
/// `ArsContext.direction` for some attribute decision.
#[test]
fn fallback_attrs_are_invariant_under_rtl_locale() {
    fn ltr_app() -> Element {
        let registries = I18nRegistries::new();

        let ctx = ArsContext::new(
            Locale::parse("en-US").expect("locale"),
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        );

        use_context_provider(|| ctx);

        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    fn rtl_app() -> Element {
        let registries = I18nRegistries::new();

        let ctx = ArsContext::new(
            // `ar-EG` is a canonical RTL locale per
            // `spec/foundation/04-internationalization.md`.
            Locale::parse("ar-EG").expect("locale"),
            Direction::Rtl,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        );

        use_context_provider(|| ctx);

        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let ltr = render_app(ltr_app);
    let rtl = render_app(rtl_app);

    // Both renders carry the canonical attrs in identical positions.
    for fragment in [
        r#"role="alert""#,
        r#"aria-live="assertive""#,
        r#"aria-atomic="true""#,
        r#"data-ars-error="true""#,
        r#"data-ars-error-count="1""#,
        r#"data-ars-scope="error-boundary""#,
        r#"data-ars-part="root""#,
    ] {
        assert!(
            ltr.contains(fragment),
            "LTR render missing {fragment}: {ltr}"
        );
        assert!(
            rtl.contains(fragment),
            "RTL render missing {fragment}: {rtl}"
        );
    }

    // The component must not emit `dir=` from inside the boundary —
    // direction is a layout-level concern handled by `ArsProvider`'s
    // root element, not by individual components.
    assert!(
        !ltr.contains(r#"dir=""#),
        "boundary must not emit dir under LTR: {ltr}"
    );
    assert!(
        !rtl.contains(r#"dir=""#),
        "boundary must not emit dir under RTL: {rtl}"
    );
}

/// Defensive smoke test for the `Debug` impls on the public types in
/// `error_boundary`. A bare `format!("{:?}", x)` would catch an
/// accidental `impl Debug` removal at compile time, but it would not
/// catch a regression that emits an empty `Debug` body. This test
/// pins the textual fingerprint by asserting each output contains the
/// type name.
#[test]
fn public_types_have_non_empty_debug_impls() {
    let messages = error_boundary::Messages::default();

    let api = error_boundary::Api::new(2);

    let messages_dbg = format!("{messages:?}");

    let api_dbg = format!("{api:?}");

    let part_dbg = format!("{:?}", error_boundary::Part::Root);

    assert!(
        messages_dbg.contains("Messages"),
        "Messages Debug impl should mention type name: {messages_dbg}"
    );
    assert!(
        api_dbg.contains("Api"),
        "Api Debug impl should mention type name: {api_dbg}"
    );
    assert!(
        part_dbg.contains("Root"),
        "Part Debug impl should mention variant name: {part_dbg}"
    );
}

/// SSR determinism check — a stand-in for a real CSR hydration
/// round-trip until that infrastructure lands (`spec/testing/07-ssr-hydration.md`).
///
/// Hydration mismatches occur when the server-rendered HTML disagrees
/// with the client's first render. The most common class of mismatch
/// is non-deterministic rendering: the boundary uses a different
/// resolved heading, different ID, or different attribute order
/// between renders. We verify the boundary is *stable* by rendering
/// the same fixture twice, in two fresh `VirtualDom` instances, and
/// asserting the output is byte-identical.
///
/// If this test starts failing, the boundary has acquired a
/// non-deterministic dependency (random IDs, system time, locale
/// derived from a side channel), and CSR hydration would mismatch the
/// SSR output. The fix is to source any non-deterministic value from
/// the surrounding `ArsProvider` or accept it as a prop.
#[test]
fn fallback_render_is_deterministic_across_two_virtualdom_instances() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let first = render_app(app);
    let second = render_app(app);

    assert_eq!(
        first, second,
        "SSR fallback output must be byte-identical across renders to avoid CSR hydration \
         mismatches; got\nfirst:\n{first}\nsecond:\n{second}"
    );
}

/// Structural accessibility contract test — the static checks an
/// `axe-core` audit would run against our fallback. The repo-wide
/// axe-core wasm integration described in
/// `spec/testing/06-accessibility-testing.md` §1 is not yet wired
/// (`tests/axe.rs` is currently a stub), so this test stands in until
/// that lands. It enforces WAI-ARIA 1.2 invariants for live alert
/// regions on the rendered HTML:
///
/// - `role="alert"` appears exactly once.
/// - `aria-live` is one of the two values that are *valid* in
///   combination with `role="alert"` (per the ARIA Authoring Practices,
///   `assertive` is the implicit default; `polite` is allowed for
///   non-blocking errors).
/// - `aria-atomic` is one of the two boolean string values; any other
///   value is a misspelling that screen readers silently ignore.
/// - All `aria-*` attribute names that appear belong to the WAI-ARIA
///   1.2 attribute set — catches typos like `aria-lable` that would
///   otherwise pass our `assert_eq!(attrs.get("aria-label"), …)` tests
///   because the typo'd attribute simply wouldn't appear in the map.
/// - There is no second `role="alert"` or `aria-live` attribute nested
///   inside the fallback subtree. Nested live regions cause assistive
///   technologies to announce content twice.
#[test]
fn fallback_satisfies_wai_aria_alert_region_contract() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html = render_app(app);

    // Exactly one alert region.
    assert_eq!(
        html.matches(r#"role="alert""#).count(),
        1,
        "expected one role=alert region, got: {html}"
    );

    // aria-live is one of the WAI-ARIA-allowed values for role=alert.
    let live_values = [r#"aria-live="assertive""#, r#"aria-live="polite""#];

    assert!(
        live_values.iter().any(|v| html.contains(v)),
        "aria-live missing or has invalid value: {html}"
    );

    // aria-atomic is a boolean per WAI-ARIA — any other value is a typo.
    let atomic_values = [r#"aria-atomic="true""#, r#"aria-atomic="false""#];

    assert!(
        atomic_values.iter().any(|v| html.contains(v)),
        "aria-atomic missing or has invalid value: {html}"
    );

    // No nested live regions inside the fallback subtree.
    assert_eq!(
        html.matches("aria-live=").count(),
        1,
        "expected exactly one aria-live, got nested: {html}"
    );

    // Every `aria-*` attribute that appears must be a known WAI-ARIA 1.2
    // attribute. The set below is the closed list of attributes we
    // currently emit, plus the ones we'd be allowed to emit. A test
    // failure here means either we added an aria-* attr without
    // updating the allowlist, or we typo'd one.
    const KNOWN_ARIA_ATTRS: &[&str] = &[
        "aria-atomic",
        "aria-busy",
        "aria-current",
        "aria-describedby",
        "aria-details",
        "aria-disabled",
        "aria-expanded",
        "aria-hidden",
        "aria-invalid",
        "aria-label",
        "aria-labelledby",
        "aria-live",
        "aria-modal",
        "aria-relevant",
        "aria-selected",
    ];

    for fragment in html.split("aria-").skip(1) {
        let attr_name = fragment
            .split(['=', ' ', '\t', '\n', '>', '/'])
            .next()
            .unwrap_or("");

        let candidate = format!("aria-{attr_name}");

        assert!(
            KNOWN_ARIA_ATTRS.contains(&candidate.as_str()),
            "unknown or misspelled aria attribute `{candidate}` in: {html}"
        );
    }
}

/// Verifies the recovery contract documented at
/// `spec/components/utility/error-boundary.md` §4 "Behavior" and the
/// Dioxus-specific path at
/// `spec/dioxus-components/utility/error-boundary.md` §8: a custom
/// fallback that calls `ctx.clear_errors()` resets the boundary so the
/// children get re-rendered. If the previously-failing child now
/// succeeds, its content shows up in place of the fallback.
///
/// Mirrors the upstream `dioxus-core` test pattern at
/// `dioxus-core/tests/error_boundary.rs::clear_error_boundary`.
#[test]
fn custom_fallback_clear_errors_restores_children_when_retry_succeeds() {
    use std::sync::atomic::{AtomicBool, Ordering};

    // First render of `ThrowsOnce` errors; subsequent renders succeed.
    static THREW_ERROR: AtomicBool = AtomicBool::new(false);

    #[component]
    fn ThrowsOnce() -> Element {
        if THREW_ERROR.load(Ordering::SeqCst) {
            rsx! { "recovered-child" }
        } else {
            THREW_ERROR.store(true, Ordering::SeqCst);

            Err(CapturedError::from_display("transient-error").into())
        }
    }

    fn app() -> Element {
        let fallback = use_callback(|ctx: ErrorContext| {
            ctx.clear_errors();

            rsx! { "" }
        });

        rsx! {
            Boundary { fallback, ThrowsOnce {} }
        }
    }

    // Reset the static so this test is order-independent within the suite.
    THREW_ERROR.store(false, Ordering::SeqCst);

    let html = render_app(app);

    assert!(
        html.contains("recovered-child"),
        "expected children to re-render after clear_errors, got: {html}"
    );
    assert!(
        !html.contains("data-ars-error"),
        "fallback markup must not be present once errors are cleared and the retry succeeds: \
         {html}"
    );
}

/// Verifies that flipping the surrounding `ArsContext.locale` signal
/// causes `Boundary` to re-resolve `Messages` from the registry on the
/// next render.
///
/// We can't observe live signal-driven re-rendering in `dioxus-ssr`
/// (each `render_app` invocation rebuilds a fresh `VirtualDom`), but we
/// can prove the dispatch is locale-driven by rendering the same
/// fixture against two distinct locales and asserting each picks the
/// correct registry entry. If the heading were resolved once at compile
/// time or pinned to a fixed locale, both renders would produce the
/// same string — so this test pins the locale-driven branching that
/// real CSR apps rely on.
#[test]
fn locale_change_in_provider_re_resolves_registered_messages() {
    /// Builds the registry the fixture installs into `ArsContext`. The
    /// English bundle is the default; the Spanish bundle is registered
    /// against the `"es"` language tag so any `es-*` locale resolves to
    /// it via the standard fallback chain.
    fn registries() -> I18nRegistries {
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("Algo salió mal."),
                },
            ),
        );

        registries
    }

    fn build_ctx(locale: Locale) -> ArsContext {
        ArsContext::new(
            locale,
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries()),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
    }

    fn app_en() -> Element {
        use_context_provider(|| build_ctx(Locale::parse("en-US").expect("locale")));

        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    fn app_es() -> Element {
        use_context_provider(|| build_ctx(Locale::parse("es-MX").expect("locale")));

        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html_en = render_app(app_en);

    assert!(
        html_en.contains("A component encountered an error."),
        "en-US locale must pick the default English bundle: {html_en}"
    );
    assert!(
        !html_en.contains("Algo salió mal."),
        "Spanish bundle must not leak under en-US: {html_en}"
    );

    let html_es = render_app(app_es);

    assert!(
        html_es.contains("Algo salió mal."),
        "es-MX locale must pick the Spanish bundle: {html_es}"
    );
    assert!(
        !html_es.contains("A component encountered an error."),
        "English default must not leak under es-MX: {html_es}"
    );
}

/// Pins the priority *order* defined at
/// `spec/components/utility/error-boundary.md` §6.1: the explicit
/// `messages` prop ALWAYS wins over a bundle registered with
/// `ArsProvider`'s `i18n_registries` for the active locale, even when
/// both are present. A regression that flipped the priority (registry
/// wins over prop) would still pass our isolated direct-prop test
/// because the registry is empty there. This test forces the
/// collision so that direction can be verified.
#[test]
fn explicit_messages_prop_wins_over_provider_registry() {
    fn app() -> Element {
        // Provider registers a Spanish bundle for the active es-MX
        // locale.
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("PROVIDER_SPANISH"),
                },
            ),
        );

        let ctx = ArsContext::new(
            Locale::parse("es-MX").expect("locale"),
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        );

        use_context_provider(|| ctx);

        // Consumer ALSO supplies an explicit `messages` prop.
        let direct = error_boundary::Messages {
            message: MessageFn::static_str("DIRECT_PROP"),
        };

        rsx! {
            Boundary { messages: direct, ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("DIRECT_PROP"),
        "explicit prop must win over registry: {html}"
    );
    assert!(
        !html.contains("PROVIDER_SPANISH"),
        "registry bundle must not leak when explicit prop is set: {html}"
    );
    assert!(
        !html.contains("A component encountered an error."),
        "default English must not leak when explicit prop is set: {html}"
    );
}

/// Verifies the full `Messages` resolution priority chain documented at
/// `spec/components/utility/error-boundary.md` §6.1:
///
/// 1. Explicit `messages` prop (covered by `custom_messages_override_default_heading`)
/// 2. Bundle registered with `ArsProvider`'s `i18n_registries` for the active locale
/// 3. `Messages::default()` fallback
///
/// This test exercises path 2 — registering a Spanish bundle in the
/// provider context and confirming `Boundary` renders the localized
/// heading without the consumer passing `messages` directly.
#[test]
fn provider_registry_messages_drive_heading_when_no_prop_override() {
    fn app() -> Element {
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("Algo salió mal."),
                },
            ),
        );

        let ctx = ArsContext::new(
            Locale::parse("es-MX").expect("locale should parse"),
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        );

        use_context_provider(|| ctx);

        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("Algo salió mal."),
        "provider-registered Spanish heading missing: {html}"
    );
    assert!(
        !html.contains("A component encountered an error."),
        "default English heading must not leak when the registry provides a localized variant: \
         {html}"
    );
}

/// Defensive coverage for the `if let Some(e) = error` branch in
/// `render_default_fallback`. The boundary normally invokes the fallback
/// only when an error is present, but `default_fallback` is also a
/// public helper that consumers may pass directly to `ErrorBoundary` —
/// which means the renderer must produce well-formed markup even when
/// the underlying `ErrorContext` is empty.
///
/// Pinning the no-error path keeps the count attribute and the empty
/// `<ul>` rendering intentional rather than incidental.
#[test]
fn default_fallback_with_empty_context_emits_zero_count_and_empty_list() {
    fn app() -> Element {
        let ctx = ErrorContext::new(None);
        default_fallback(ctx)
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-error="true""#),
        "missing data-ars-error: {html}"
    );
    assert!(
        html.contains(r#"data-ars-error-count="0""#),
        "expected count=0 for empty error context: {html}"
    );

    // The static heading still renders — the alert region is structurally
    // identical, just empty of items.
    assert!(
        html.contains("A component encountered an error."),
        "static heading missing on no-error path: {html}"
    );

    // No `<li>` should be emitted on the no-error branch.
    assert!(
        !html.contains(r#"data-ars-part="item""#),
        "expected no item entries on no-error path: {html}"
    );
}

/// XSS / HTML-escape contract: error `Display` text rendered into each
/// `<li>` must be escaped by the framework so consumer-provided error
/// strings cannot inject markup.
///
/// This test guards against a regression where someone refactors the
/// renderer to use a raw-HTML emission path (e.g. `dangerous_inner_html`)
/// — which would let any error message containing `<script>` or
/// `<img onerror=...>` execute as live markup.
///
/// The contract: the literal angle-bracket sequence `<script>` does NOT
/// appear in the rendered HTML; it must be escaped to `&lt;script&gt;`
/// (or an equivalent serialization).
#[test]
fn error_message_text_is_html_escaped_in_fallback_list() {
    #[component]
    fn ScriptyChild() -> Element {
        Err(CapturedError::from_display("<script>alert('xss')</script>").into())
    }

    fn app() -> Element {
        rsx! {
            Boundary { ScriptyChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("<script>alert('xss')</script>"),
        "error string was rendered as raw HTML — XSS vector! got: {html}"
    );

    // Either named (`&lt;script&gt;`) or numeric (`&#60;script&#62;`)
    // escaping is acceptable; what matters is that the raw script tag
    // does not survive into the rendered output.
    assert!(
        html.contains("&lt;script&gt;")
            || html.contains("&lt;script")
            || html.contains("&#60;script&#62;")
            || html.contains("&#x3c;script&#x3e;"),
        "error string should appear escaped (named or numeric entities): {html}"
    );
}

/// UTF-8 / non-ASCII error text must round-trip through the renderer
/// without truncation, mojibake, or escaping artifacts. Spec §6 declares
/// that error strings come from `Display` and are not translated, so any
/// locale's exception messages must render verbatim.
#[test]
fn non_ascii_error_text_is_preserved_through_render() {
    #[component]
    fn JapaneseErrorChild() -> Element {
        Err(CapturedError::from_display("値が無効です").into())
    }

    fn app() -> Element {
        rsx! {
            Boundary { JapaneseErrorChild {} }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("値が無効です"),
        "non-ASCII error text was lost or mangled: {html}"
    );
}

/// `insta` snapshot of the rendered HTML on the **happy path** (no
/// caught errors). Pinned so a future Dioxus release that quietly
/// changes attribute ordering, escape style, or void-tag form is
/// caught by a snapshot diff rather than slipping through every
/// `.contains(...)` substring check.
///
/// Companion to the `error_boundary_html_snapshot_error` test below
/// and to the AttrMap-level snapshots in
/// `ars-components/src/utility/error_boundary.rs`.
#[test]
fn error_boundary_html_snapshot_happy_path() {
    fn app() -> Element {
        rsx! {
            Boundary {
                p { "child-ok" }
            }
        }
    }

    insta::assert_snapshot!("dioxus_error_boundary_happy_path", render_app(app));
}

/// `insta` snapshot of the rendered HTML on the **error path** —
/// fallback markup including localized heading and one error item.
#[test]
fn error_boundary_html_snapshot_error_path() {
    fn app() -> Element {
        rsx! {
            Boundary { ThrowingChild {} }
        }
    }

    insta::assert_snapshot!("dioxus_error_boundary_error_path", render_app(app));
}

/// Nested-Boundary contract: when an inner `Boundary` catches an error,
/// the outer `Boundary` stays idle (no fallback markup, children render
/// normally). This guards against an aria-live double-announcement that
/// would arise if both boundaries fired their fallbacks for the same
/// caught error.
#[test]
fn inner_boundary_catches_outer_stays_idle() {
    fn app() -> Element {
        rsx! {
            Boundary {
                p { "outer-sibling" }
                Boundary { ThrowingChild {} }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("outer-sibling"),
        "outer Boundary's non-erroring sibling should render: {html}"
    );

    // Exactly one fallback: only the inner boundary triggered.
    let fallback_opens = html.matches(r#"data-ars-error="true""#).count();

    assert_eq!(
        fallback_opens, 1,
        "inner Boundary should swallow the error; outer should stay idle. \
         Saw {fallback_opens} fallback container(s) in: {html}"
    );

    let alert_roles = html.matches(r#"role="alert""#).count();

    assert_eq!(
        alert_roles, 1,
        "exactly one alert region expected (the inner one); got {alert_roles}: {html}"
    );
}
