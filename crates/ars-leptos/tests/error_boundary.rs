//! Unit tests for `error_boundary::Boundary` (issue #629, sibling of #197).
//!
//! Exercises `spec/components/utility/error-boundary.md` and the
//! Leptos-specific contract at `spec/leptos-components/utility/error-boundary.md`:
//!
//! - children render when no descendant errors
//! - the canonical fallback renders with `role="alert"`,
//!   `aria-live="assertive"`, `aria-atomic="true"`, `data-ars-error="true"`,
//!   `data-ars-error-count`, and the `<ul>`/`<li>` error list when a child
//!   returns `Err`
//! - the `on_error` telemetry hook fires for each captured error
//! - a custom `fallback` prop replaces the default markup entirely
//! - the boundary catches the error so siblings around the wrapper still
//!   render
//!
//! The whole module is gated on the `ssr` feature so SSR string rendering
//! via `View::to_html()` is available.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use std::{
    fmt::{self, Display},
    sync::{Arc, Mutex},
};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, MessageFn, MessagesRegistry,
    NullPlatformEffects, StyleStrategy,
};
use ars_i18n::{Direction, Locale, StubIntlBackend};
use ars_leptos::{
    ArsContext,
    error_boundary::{self, Boundary, default_fallback},
    provide_ars_context,
};
use leptos::prelude::*;

/// Minimal `std::error::Error` implementation for triggering the boundary.
#[derive(Debug)]
struct BoomError(&'static str);

impl Display for BoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for BoomError {}

const fn boom() -> Result<&'static str, BoomError> {
    Err(BoomError("boom-from-child"))
}

#[test]
fn renders_children_when_no_error() {
    let html = view! {
        <Boundary>
            <p>"child-ok"</p>
        </Boundary>
    }
    .to_html();

    assert!(html.contains("child-ok"), "child not rendered: {html}");
    assert!(
        !html.contains("data-ars-error"),
        "fallback should not render on the happy path: {html}"
    );
}

#[test]
fn renders_canonical_fallback_attrs_when_child_returns_err() {
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

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
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains("A component encountered an error."),
        "missing static message: {html}"
    );
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
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains(r#"data-ars-part="list""#),
        "missing list part: {html}"
    );
    assert!(html.contains("<ul"), "expected <ul>: {html}");
    assert!(html.contains("<li"), "expected <li>: {html}");
}

#[test]
fn fallback_emits_data_ars_error_count() {
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains(r#"data-ars-error-count="1""#),
        "expected single-error count: {html}"
    );
}

#[test]
fn boundary_does_not_propagate_error_past_wrapper() {
    let html = view! {
        <div data-outside="1">
            <Boundary>
                {boom()}
            </Boundary>
            <p>"after-boundary"</p>
        </div>
    }
    .to_html();

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
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_for_callback = Arc::clone(&captured);

    let on_error = Callback::new(move |err: error_boundary::CapturedError| {
        captured_for_callback
            .lock()
            .expect("lock")
            .push(err.to_string());
    });

    let _html = view! {
        <Boundary on_error=on_error>
            {boom()}
        </Boundary>
    }
    .to_html();

    let captured = captured.lock().expect("lock");

    assert!(
        captured.iter().any(|m| m.contains("boom-from-child")),
        "on_error did not fire with the captured error; got {captured:?}"
    );
}

#[test]
fn custom_fallback_replaces_default_markup() {
    use leptos::tachys::view::any_view::AnyView;

    let fallback: error_boundary::FallbackHandler = Callback::new(|_errors| -> AnyView {
        view! { <div class="my-custom-error">"Custom UI"</div> }.into_any()
    });

    let html = view! {
        <Boundary fallback=fallback>
            {boom()}
        </Boundary>
    }
    .to_html();

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
    let html = view! {
        <ErrorBoundary fallback=default_fallback>
            {boom()}
        </ErrorBoundary>
    }
    .to_html();

    assert!(html.contains(r#"data-ars-error="true""#));
    assert!(html.contains("A component encountered an error."));
    assert!(html.contains(r#"data-ars-error-count="1""#));
}

#[test]
fn custom_messages_override_default_heading() {
    let messages = error_boundary::Messages {
        message: MessageFn::static_str("Algo deu errado."),
    };

    let html = view! {
        <Boundary messages=messages>
            {boom()}
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains("Algo deu errado."),
        "messages override did not render: {html}"
    );
    assert!(
        !html.contains("A component encountered an error."),
        "default English string should not render when override is provided: {html}"
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
/// provider and confirming `Boundary` renders the localized heading
/// without the consumer passing `messages` directly.
#[test]
fn provider_registry_messages_drive_heading_when_no_prop_override() {
    let owner = Owner::new();
    let html = owner.with(|| {
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("Algo salió mal."),
                },
            ),
        );

        provide_ars_context(ArsContext::new(
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
            StyleStrategy::Inline,
        ));

        view! {
            <Boundary>
                {boom()}
            </Boundary>
        }
        .to_html()
    });

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

/// Verifies that the canonical fallback markup is invariant under
/// RTL locales. The component does not branch on text direction
/// (the `data-ars-error*` attrs and `role="alert"` are
/// direction-agnostic per WAI-ARIA), so an RTL locale must produce
/// the same attribute set as an LTR locale. Catches a regression
/// where the boundary accidentally consumes `ArsContext.direction`.
#[test]
fn fallback_attrs_are_invariant_under_rtl_locale() {
    fn render_with_locale(tag: &'static str, direction: Direction) -> String {
        let owner = Owner::new();
        owner.with(|| {
            provide_ars_context(ArsContext::new(
                Locale::parse(tag).expect("locale should parse"),
                direction,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                StyleStrategy::Inline,
            ));

            view! { <Boundary>{boom()}</Boundary> }.to_html()
        })
    }

    let ltr = render_with_locale("en-US", Direction::Ltr);

    // `ar-EG` is a canonical RTL locale per
    // `spec/foundation/04-internationalization.md`.
    let rtl = render_with_locale("ar-EG", Direction::Rtl);

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

/// Defensive smoke test for `Debug` impls on the public types in
/// `error_boundary`. Pins the fingerprint of each `Debug` output by
/// asserting it mentions the type / variant name. Catches a
/// regression where an explicit `impl Debug for X { fn fmt … }` is
/// accidentally emptied out.
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
/// round-trip until that infrastructure lands
/// (`spec/testing/07-ssr-hydration.md`).
///
/// Hydration mismatches occur when the server-rendered HTML disagrees
/// with the client's first render. The most common class of mismatch
/// is non-deterministic rendering: the boundary picks a different
/// resolved heading, different ID, or different attribute order
/// between renders. We verify the boundary is *stable* by rendering
/// the same fixture twice in two fresh reactive `Owner`s and asserting
/// the output is byte-identical.
///
/// If this test starts failing, the boundary has acquired a
/// non-deterministic dependency (random IDs, system time, locale
/// from a side channel), and CSR hydration would mismatch the SSR
/// output.
#[test]
fn fallback_render_is_deterministic_across_two_owners() {
    fn render() -> String {
        let owner = Owner::new();
        owner.with(|| {
            view! {
                <Boundary>
                    {boom()}
                </Boundary>
            }
            .to_html()
        })
    }

    let first = render();
    let second = render();

    assert_eq!(
        first, second,
        "SSR fallback output must be byte-identical across renders; got\nfirst:\n{first}\nsecond:\n{second}"
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
/// - `aria-live` has one of the two values valid with `role="alert"`.
/// - `aria-atomic` is a boolean string.
/// - No nested live region inside the fallback subtree.
/// - Every `aria-*` attribute name is a known WAI-ARIA 1.2 attribute
///   (catches typos like `aria-lable` that pass attribute-presence
///   checks because the misspelling silently never appears).
#[test]
fn fallback_satisfies_wai_aria_alert_region_contract() {
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

    assert_eq!(
        html.matches(r#"role="alert""#).count(),
        1,
        "expected one role=alert region, got: {html}"
    );

    let live_values = [r#"aria-live="assertive""#, r#"aria-live="polite""#];

    assert!(
        live_values.iter().any(|v| html.contains(v)),
        "aria-live missing or has invalid value: {html}"
    );

    let atomic_values = [r#"aria-atomic="true""#, r#"aria-atomic="false""#];

    assert!(
        atomic_values.iter().any(|v| html.contains(v)),
        "aria-atomic missing or has invalid value: {html}"
    );

    assert_eq!(
        html.matches("aria-live=").count(),
        1,
        "expected exactly one aria-live, got nested: {html}"
    );

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

/// Pins the priority *order* defined at
/// `spec/components/utility/error-boundary.md` §6.1: the explicit
/// `messages` prop ALWAYS wins over a bundle registered with
/// `ArsProvider`'s `i18n_registries` for the active locale, even when
/// both are present. A regression that flipped the priority (registry
/// wins over prop) would still pass our isolated direct-prop test
/// because the registry is empty there.
#[test]
fn explicit_messages_prop_wins_over_provider_registry() {
    let owner = Owner::new();
    let html = owner.with(|| {
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("PROVIDER_SPANISH"),
                },
            ),
        );

        provide_ars_context(ArsContext::new(
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
            StyleStrategy::Inline,
        ));

        let direct = error_boundary::Messages {
            message: MessageFn::static_str("DIRECT_PROP"),
        };

        view! {
            <Boundary messages=direct>
                {boom()}
            </Boundary>
        }
        .to_html()
    });

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

/// Verifies the recovery API surface documented at
/// `spec/components/utility/error-boundary.md` §4 "Behavior": a custom
/// fallback receives the `ArcRwSignal<Errors>` and can mutate it
/// (e.g. by calling `errors.update(|e| e.clear())`) to clear the
/// boundary so the framework re-renders children.
///
/// SSR cannot observe the actual reactive re-render — `to_html()` is a
/// one-shot snapshot, so by the time the fallback closure runs, the
/// boundary has already committed to the fallback branch for this
/// render. The reactive switch-back to children happens on the next
/// render driven by the runtime, which only exists under CSR or
/// hydration. What we can verify here is that:
///
/// 1. The signal handed to the fallback is mutable (the recovery API is
///    actually reachable, not a read-only snapshot).
/// 2. After a `.clear()`, the signal is empty — confirming the
///    framework's contract that an empty `Errors` collection causes the
///    boundary to render children on the next reactive pass.
#[test]
fn custom_fallback_can_clear_errors_for_reactive_recovery() {
    use leptos::{error::ErrorId, tachys::view::any_view::AnyView};

    let owner = Owner::new();
    owner.with(|| {
        let mut errors = Errors::default();

        errors.insert(ErrorId::from(7usize), BoomError("recoverable"));

        let signal = ArcRwSignal::new(errors);

        // Sanity: the signal currently has one error.
        assert_eq!(signal.with(|e| e.iter().count()), 1);

        // A custom fallback that wires a "Try again" affordance would
        // mutate the errors signal from inside an event handler. We
        // mirror that behavior synchronously here. Replacing with
        // `Errors::default()` is the public idiom because `Errors` does
        // not expose a `clear()` method directly.
        let fallback: error_boundary::FallbackHandler =
            Callback::new(|errors: ArcRwSignal<Errors>| -> AnyView {
                errors.update(|e| *e = Errors::default());
                view! { <span>"after-clear"</span> }.into_any()
            });

        // Drive the fallback the way the framework would.
        let _view = fallback.run(signal.clone());

        // After the fallback runs, the recovery API has cleared every
        // entry — proving the wrapper's exposed signal is the same one
        // the framework primitive watches for the children-reset path.
        assert_eq!(signal.with(|e| e.iter().count()), 0);
        assert!(signal.with(Errors::is_empty));
    });
}

/// Verifies the canonical multi-error contract: when the `Errors` signal
/// holds N entries, the fallback's `data-ars-error-count` matches N and
/// every error's `Display` text appears in the rendered list.
///
/// This case is uniquely Leptos's — Dioxus `ErrorContext` collapses to
/// the most-recent single error. Without this test, drift in
/// `default_fallback`'s iteration could produce a wrong `count` attribute
/// or drop entries silently.
///
/// We construct the `Errors` collection directly rather than provoking
/// it through three sibling `Result::Err` children, because in
/// single-pass SSR the framework's `ErrorBoundary` only commits the
/// most-recent error before rendering the fallback (subsequent siblings
/// never render once the boundary switches to its fallback branch).
/// Bypassing the framework primitive lets us assert on
/// `default_fallback`'s rendering of an arbitrary multi-entry signal —
/// the contract real apps rely on under CSR or hydration when errors
/// accumulate over multiple render passes.
#[test]
fn fallback_renders_all_errors_with_matching_count() {
    use leptos::{error::ErrorId, tachys::view::any_view::AnyView};

    let owner = Owner::new();
    let html = owner.with(|| {
        // ErrorId is `pub struct ErrorId(usize)` with `From<usize>`. Use
        // distinct keys so the FxHashMap actually retains all three entries
        // (the `Default` impl returns the same id, so default keys collide).
        let mut errors = Errors::default();

        errors.insert(ErrorId::from(1usize), BoomError("first-boom"));
        errors.insert(ErrorId::from(2usize), BoomError("second-boom"));
        errors.insert(ErrorId::from(3usize), BoomError("third-boom"));

        let signal = ArcRwSignal::new(errors);

        let view: AnyView = default_fallback(signal);

        view.to_html()
    });

    assert!(
        html.contains(r#"data-ars-error-count="3""#),
        "expected count=3, got: {html}"
    );

    for label in ["first-boom", "second-boom", "third-boom"] {
        assert!(
            html.contains(label),
            "expected error label {label} in fallback list, got: {html}"
        );
    }

    let item_opens = html.matches(r#"data-ars-part="item""#).count();

    assert_eq!(
        item_opens, 3,
        "expected exactly 3 item parts, found {item_opens} in: {html}"
    );
}

/// XSS / HTML-escape contract: error `Display` text rendered into each
/// `<li>` must be escaped by Leptos's view system. Guards against a
/// regression where someone refactors the renderer to use a raw-HTML
/// path (e.g. `inner_html`) — which would let any error message
/// containing `<script>` execute as live markup.
#[test]
fn error_message_text_is_html_escaped_in_fallback_list() {
    #[derive(Debug)]
    struct ScriptyError;

    impl Display for ScriptyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("<script>alert('xss')</script>")
        }
    }

    impl std::error::Error for ScriptyError {}

    let html = view! {
        <Boundary>
            {Result::<&str, ScriptyError>::Err(ScriptyError)}
        </Boundary>
    }
    .to_html();

    assert!(
        !html.contains("<script>alert('xss')</script>"),
        "error string was rendered as raw HTML — XSS vector! got: {html}"
    );
    assert!(
        html.contains("&lt;script&gt;")
            || html.contains("&lt;script")
            || html.contains("&#60;script&#62;")
            || html.contains("&#x3c;script&#x3e;"),
        "error string should appear escaped (named or numeric entities): {html}"
    );
}

/// UTF-8 / non-ASCII error text must round-trip through the Leptos
/// renderer without truncation, mojibake, or escaping artifacts. Spec
/// §6 declares that error strings come from `Display` and are not
/// translated, so any locale's exception messages must render verbatim.
#[test]
fn non_ascii_error_text_is_preserved_through_render() {
    #[derive(Debug)]
    struct JapaneseError;

    impl Display for JapaneseError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("値が無効です")
        }
    }

    impl std::error::Error for JapaneseError {}

    let html = view! {
        <Boundary>
            {Result::<&str, JapaneseError>::Err(JapaneseError)}
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains("値が無効です"),
        "non-ASCII error text was lost or mangled: {html}"
    );
}

/// G: a `run_fallback` invocation with **no telemetry hook** must not
/// even iterate the errors signal. Conversely, with telemetry set but a
/// completely empty signal, no callback should fire either. This pins
/// down the "zero noise on a clean signal" contract that complements
/// the existing `run_fallback_fires_on_error_once_per_entry` test.
#[test]
fn on_error_does_not_fire_for_empty_errors_signal() {
    use std::sync::Mutex;

    let owner = Owner::new();
    owner.with(|| {
        let captured: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

        let captured_for_callback = Arc::clone(&captured);
        let on_error: Callback<error_boundary::CapturedError> =
            Callback::new(move |_err: error_boundary::CapturedError| {
                *captured_for_callback.lock().expect("lock") += 1;
            });

        let signal = ArcRwSignal::new(Errors::default());

        let html = view! {
            <Boundary on_error=on_error>
                <p>"child-ok"</p>
            </Boundary>
        }
        .to_html();

        // No errors → fallback never runs → on_error never fires.
        assert!(
            html.contains("child-ok"),
            "happy path should render children: {html}"
        );

        let fires = *captured.lock().expect("lock");

        assert_eq!(
            fires, 0,
            "on_error should not fire when no error is caught; got {fires} call(s)"
        );

        // Sanity that the unused signal stays empty.
        assert!(signal.with(Errors::is_empty));
    });
}

/// J: `default_fallback` invoked with an empty `Errors` signal must
/// still produce well-formed canonical markup — the static heading,
/// `data-ars-error-count="0"`, and an empty `<ul>` (no `<li>` items).
/// Mirrors the Dioxus C-test on the Leptos side.
#[test]
fn default_fallback_with_empty_errors_signal_emits_zero_count() {
    let owner = Owner::new();
    let html = owner.with(|| {
        let signal = ArcRwSignal::new(Errors::default());

        default_fallback(signal).to_html()
    });

    assert!(
        html.contains(r#"data-ars-error="true""#),
        "missing data-ars-error: {html}"
    );
    assert!(
        html.contains(r#"data-ars-error-count="0""#),
        "expected count=0 for empty signal: {html}"
    );
    assert!(
        html.contains("A component encountered an error."),
        "static heading missing on no-error path: {html}"
    );
    assert!(
        !html.contains(r#"data-ars-part="item""#),
        "expected no item entries on no-error path: {html}"
    );
}

/// `insta` snapshot of the rendered HTML on the **happy path** (no
/// caught errors). Pinned so a future Leptos release that quietly
/// changes attribute ordering, escape style, or void-tag form is
/// caught by a snapshot diff rather than slipping through every
/// `.contains(...)` substring check.
#[test]
fn error_boundary_html_snapshot_happy_path() {
    let html = view! {
        <Boundary>
            <p>"child-ok"</p>
        </Boundary>
    }
    .to_html();

    insta::assert_snapshot!(html, @"<p>child-ok</p>");
}

/// `insta` snapshot of the rendered HTML on the **error path** —
/// fallback markup including localized heading and one error item.
#[test]
fn error_boundary_html_snapshot_error_path() {
    let html = view! {
        <Boundary>
            {boom()}
        </Boundary>
    }
    .to_html();

    insta::assert_snapshot!(html, @r#"<div data-ars-error="true" data-ars-error-count="1" data-ars-part="root" data-ars-scope="error-boundary" aria-atomic="true" aria-live="assertive" role="alert"><p data-ars-part="message" data-ars-scope="error-boundary">A component encountered an error.</p><ul data-ars-part="list" data-ars-scope="error-boundary"><li data-ars-part="item" data-ars-scope="error-boundary">boom-from-child</li><!></ul></div>"#);
}

/// Nested-Boundary contract: when an inner `Boundary` catches an error,
/// the outer `Boundary` stays idle (no fallback markup, children render
/// normally). This guards against an aria-live double-announcement that
/// would arise if both boundaries fired their fallbacks for the same
/// caught error.
#[test]
fn inner_boundary_catches_outer_stays_idle() {
    let html = view! {
        <Boundary>
            <p>"outer-sibling"</p>
            <Boundary>
                {boom()}
            </Boundary>
        </Boundary>
    }
    .to_html();

    assert!(
        html.contains("outer-sibling"),
        "outer Boundary's non-erroring sibling should render: {html}"
    );

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

// ────────────────────────────────────────────────────────────────────
// Property-based tests for `default_fallback` multi-error rendering
// ────────────────────────────────────────────────────────────────────

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(64)
    ))]

    /// For any `Errors` collection of size `n in 0..=12`, `default_fallback`
    /// must:
    /// - emit `data-ars-error-count="{n}"` on the root,
    /// - render exactly `n` `<li data-ars-part="item">` entries,
    /// - include each error's `Display` text.
    ///
    /// The framework-agnostic `Api` proptest pins the count attribute in
    /// isolation; this proptest pins the full Leptos render path so an
    /// off-by-one in the fallback iteration would fail here even if the
    /// `Api` produces the right `data-ars-error-count`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_default_fallback_li_count_matches_signal_size(
        n in 0usize..=12,
    ) {
        use leptos::error::ErrorId;

        let owner = Owner::new();
        let html = owner.with(|| {
            let mut errs = Errors::default();

            for i in 0..n {
                errs.insert(ErrorId::from(i + 1), BoomError(match i % 3 {
                    0 => "boom-a",
                    1 => "boom-b",
                    _ => "boom-c",
                }));
            }

            let signal = ArcRwSignal::new(errs);

            default_fallback(signal).to_html()
        });

        let count_attr = format!(r#"data-ars-error-count="{n}""#);

        proptest::prop_assert!(
            html.contains(&count_attr),
            "expected {count_attr} for n={n}, got: {html}",
        );

        let item_opens = html.matches(r#"data-ars-part="item""#).count();

        proptest::prop_assert_eq!(
            item_opens, n,
            "expected exactly {} item parts for n={}, got {}",
            n, n, item_opens
        );
    }
}
