# Testing Overview

## 1. Overview

This document defines the testing strategy for **ars-ui** components. The goal is comprehensive, automated verification of every state machine, its ARIA contract, its adapter integration, and its accessibility compliance. Tests are organized in three tiers:

1. **Unit tests** — pure Rust, no DOM, exercising `transition()` logic.
2. **Integration tests** — exercising the `Service` layer (send → transition → effect → event loop).
3. **Adapter tests** — mounting real components in Leptos/Dioxus and asserting DOM output.

> **Note:** Snapshot tests (03-snapshot-tests.md) are a cross-cutting verification layer applied within unit and integration tiers, not a standalone tier.

All tests run in CI on every pull request. Accessibility audits run nightly.

### 1.1 Testing Tiers

The three testing tiers are intentionally distinct:

1. **Unit tests** validate pure machine transition logic with no runtime or DOM.
2. **Integration tests** validate `Service` behavior, effect draining, and follow-up event sequencing.
3. **Adapter tests** validate rendered DOM output and interaction wiring through the framework adapters.

Snapshot assertions are a cross-cutting technique used within these tiers, not a fourth tier.

### 1.2 Testing Spec Files

| File                         | Description                                       |
| ---------------------------- | ------------------------------------------------- |
| 00-overview                  | Testing strategy and quick start                  |
| 01-unit-tests                | Table-driven transition testing                   |
| 02-integration-tests         | Service layer and effect testing                  |
| 03-snapshot-tests            | AttrMap regression testing with insta             |
| 04-aria-helpers              | Reusable ARIA assertion helpers                   |
| 05-adapter-harness           | Leptos and Dioxus test patterns                   |
| 06-accessibility-testing     | axe-core and screen reader testing                |
| 07-ssr-hydration             | SSR output and hydration correctness              |
| 08-i18n-testing              | RTL, locale switching, IME composition            |
| 09-state-machine-correctness | Systematic defect detection                       |
| 10-keyboard-focus            | Presence lifecycle, keyboard, disabled states     |
| 11-form-validation           | Form integration and validation testing           |
| 12-advanced                  | Timer, canvas, file, clipboard, visual regression |
| 13-policies                  | Test organization, naming, feature flags          |
| 14-ci                        | CI/CD pipeline and enforcement                    |
| 15-test-harness              | TestHarness API definition                        |

---

> **Note:** Test examples assume component `Props` types implement `Default`. Each component must provide its own `Default` impl for tests to compile.

## 2. Testing Quick Start

Minimal examples to get started with each testing tier.

### 2.1 Example 1: Test a core state machine

Create a machine, send an event, and assert the resulting state — no DOM or adapter needed.

```rust
#[test]
fn toggle_transitions_to_pressed() {
    let props = toggle::Props::default();
    let (state, ctx) = toggle::Machine::init(&props);
    assert_eq!(state, toggle::State::Off);

    // Machine::transition takes 4 args: (state, event, ctx, props) → returns Option<TransitionPlan>
    let plan = toggle::Machine::transition(&state, &toggle::Event::Toggle, &ctx, &props)
        .expect("expected transition");
    assert_eq!(plan.target, Some(toggle::State::On));
}
```

### 2.2 Example 2: Test a Leptos adapter component

Mount a real component, query the DOM, and assert output using raw framework utilities.

```rust
#[wasm_bindgen_test]
fn dialog_opens_on_trigger_click() {
    mount_to_body(|| {
        view! { <Dialog id="d1"><p>"Hello"</p></Dialog> }
    });

    let trigger = document()
        .query_selector("[data-ars-part='trigger']")
        .expect("query must not error")
        .expect("trigger element must exist");
    trigger.unchecked_ref::<web_sys::HtmlElement>().click();

    let content = document()
        .query_selector("[data-ars-part='content']")
        .expect("query must not error")
        .expect("content element must exist");
    assert_eq!(content.get_attribute("data-state").as_deref(), Some("open"));
    assert_eq!(content.get_attribute("role").as_deref(), Some("dialog"));
}
```

### 2.3 Example 3: Test accessibility (Leptos)

Check ARIA attributes on a mounted component using `query_selector`.

```rust
#[wasm_bindgen_test]
fn checkbox_has_correct_aria() {
    mount_to_body(|| {
        view! { <Checkbox id="cb1" /> }
    });

    let control = document()
        .query_selector("[data-ars-part='control']")
        .expect("query must not error")
        .expect("control element must exist");
    assert_eq!(control.get_attribute("role").as_deref(), Some("checkbox"));
    assert_eq!(control.get_attribute("aria-checked").as_deref(), Some("false"));
}
```

### 2.4 Example 4: Test a Dioxus adapter component

Use Dioxus 0.7.3 testing patterns. The `dioxus_ssr::render_vdom()` function and
`dom.handle_event()` are deprecated. Use the current SSR API and the
`dioxus-testing` crate for event simulation.

```rust
#[test]
fn dialog_renders_with_correct_role() {
    // Dioxus 0.7.3: use dioxus::ssr::render_element() for SSR rendering
    // Note: verify function name against Dioxus 0.7.3 docs
    let html = dioxus::ssr::render_element(rsx! {
        Dialog { id: "d1", p { "Hello" } }
    });
    assert!(html.contains(r#"role="dialog""#));
}
```

## 3. Testing File Index

| File                                                               | Topic                        |
| ------------------------------------------------------------------ | ---------------------------- |
| [00-overview.md](00-overview.md)                                   | Testing strategy overview    |
| [01-unit-tests.md](01-unit-tests.md)                               | Unit testing patterns        |
| [02-integration-tests.md](02-integration-tests.md)                 | Integration testing patterns |
| [03-snapshot-tests.md](03-snapshot-tests.md)                       | Snapshot testing patterns    |
| [04-aria-helpers.md](04-aria-helpers.md)                           | ARIA assertion helpers       |
| [05-adapter-harness.md](05-adapter-harness.md)                     | Adapter test harness         |
| [06-accessibility-testing.md](06-accessibility-testing.md)         | Accessibility testing        |
| [07-ssr-hydration.md](07-ssr-hydration.md)                         | SSR and hydration testing    |
| [08-i18n-testing.md](08-i18n-testing.md)                           | Internationalization testing |
| [09-state-machine-correctness.md](09-state-machine-correctness.md) | State machine correctness    |
| [10-keyboard-focus.md](10-keyboard-focus.md)                       | Keyboard and focus testing   |
| [11-form-validation.md](11-form-validation.md)                     | Form validation testing      |
| [12-advanced.md](12-advanced.md)                                   | Advanced testing patterns    |
| [13-policies.md](13-policies.md)                                   | Testing policies             |
| [14-ci.md](14-ci.md)                                               | CI infrastructure            |
| [15-test-harness.md](15-test-harness.md)                           | Test harness API             |

---
