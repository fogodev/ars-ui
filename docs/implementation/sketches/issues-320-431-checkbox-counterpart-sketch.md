# Component Adapter Reference Exploration Sketch

## Task

- Issues: #320, #431
- Component: Checkbox
- Category: input
- Adapters in scope: Leptos, Dioxus
- Specs read: `spec/leptos-components/input/checkbox.md`, `spec/dioxus-components/input/checkbox.md`, `docs/implementation/adapter-component-delivery.md`
- Date: 2026-06-07

## Reference Sources

- Primary counterpart: React Aria Checkbox / React Spectrum Checkbox styling examples
- Primary URL: <https://react-aria.adobe.com/Checkbox>
- Fallback counterparts inspected: Not needed before coding; React Aria covers the relevant checkbox, form, required, validation, focus, and controlled/uncontrolled axes.
- Reason for fallback or N/A: Ark UI / Chakra UI and Radix UI / shadcn/ui remain fallbacks only if a later audit finds a state axis not demonstrated by React Aria.

## Playwright Exploration Commands

```bash
playwright-cli -s=reference open https://react-spectrum.adobe.com/react-aria/Checkbox.html
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-checkbox-initial.yml
playwright-cli -s=reference eval "async () => { const box = document.querySelector('input[type=checkbox]'); box.scrollIntoView({block:'center'}); box.focus(); const before = {checked: box.checked, focusVisible: box.matches(':focus-visible'), outline: getComputedStyle(box).outline, accentColor: getComputedStyle(box).accentColor, rect: box.getBoundingClientRect().toJSON?.() || {...box.getBoundingClientRect()}}; box.click(); const afterClick = {checked: box.checked, focusVisible: box.matches(':focus-visible'), outline: getComputedStyle(box).outline, accentColor: getComputedStyle(box).accentColor}; return {before, afterClick, consoleErrors: []}; }"
playwright-cli -s=reference eval "async () => { const labels = [...document.querySelectorAll('label')]; const label = labels.find(l => /Accept terms/.test(l.textContent || '')); const input = label?.querySelector('input[type=checkbox]'); const form = input?.closest('form'); const button = form?.querySelector('button,[type=submit]'); const before = input ? {checked: input.checked, required: input.required, invalid: input.matches(':invalid'), validationMessage: input.validationMessage, ariaInvalid: input.getAttribute('aria-invalid')} : null; button?.click(); await new Promise(r => setTimeout(r, 50)); const afterSubmit = input ? {checked: input.checked, required: input.required, invalid: input.matches(':invalid'), validationMessage: input.validationMessage, ariaInvalid: input.getAttribute('aria-invalid'), activeTag: document.activeElement?.tagName, activeText: document.activeElement?.closest('label')?.textContent?.trim()} : null; input?.click(); await new Promise(r => setTimeout(r, 50)); const afterCheck = input ? {checked: input.checked, invalid: input.matches(':invalid'), validationMessage: input.validationMessage, ariaInvalid: input.getAttribute('aria-invalid')} : null; return {before, afterSubmit, afterCheck, formText: form?.textContent?.replace(/\s+/g,' ').trim().slice(0,400)}; }"
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-checkbox-after-required-submit.yml
```

## Reference Evidence

| State or outcome  | Command or action                           | Artifact                                                       | Notes                                                                                           |
| ----------------- | ------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Basic             | Open React Aria Checkbox page               | `.playwright-cli/reference-checkbox-initial.yml`               | Page exposes labelled checkbox examples and a required form example.                            |
| Focused           | Focus first native checkbox through `eval`  | eval output                                                    | Focus-visible outline reported as `rgb(0, 95, 204) auto 1px`.                                   |
| Checked           | Click first checkbox through `eval`         | eval output                                                    | `checked` changes from `false` to `true`.                                                       |
| Required invalid  | Submit required terms form while unchecked  | `.playwright-cli/reference-checkbox-after-required-submit.yml` | Required input remains unchecked, `:invalid` true, `aria-invalid="true"`, focus moves to input. |
| Required recovery | Click required checkbox after failed submit | eval output                                                    | `checked=true`, `:invalid=false`, `aria-invalid` clears.                                        |

## Observed Reference Outcomes

| Axis              | Reference behavior                                                  | User-visible outcome                                    | Accessibility outcome                                                    | Notes                                                                                      |
| ----------------- | ------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| Basic rendering   | Label wraps a native checkbox input.                                | Small square control with text label.                   | Native checkbox semantics and accessible name from label.                | ars-ui uses a role-owning visual control plus hidden input, an idiomatic equivalent.       |
| Checked/unchecked | Click toggles checked property.                                     | Native checkmark appears or clears.                     | Checked state is exposed by native checkbox state.                       | ars-ui exposes `aria-checked` on `Control` and hidden input checked only when checked.     |
| Indeterminate     | React Aria documents controlled `isIndeterminate`.                  | Mixed visual mark.                                      | Mixed state exposed as `aria-checked="mixed"` where custom role is used. | Must preserve controlled indeterminate until parent updates.                               |
| Disabled          | Disabled checkbox blocks mutation and dims visually.                | Control is visibly disabled.                            | Disabled state is exposed by native disabled or `aria-disabled`.         | ars-ui must set disabled attrs and block pointer/keyboard mutation.                        |
| Readonly          | React Aria Checkbox does not make readonly a primary checkbox axis. | N/A in primary reference.                               | N/A.                                                                     | ars-ui supports readonly as an intentional extension; keep focusable and block mutation.   |
| Required          | Required checkbox participates in form validity.                    | Submit while unchecked shows invalid feedback.          | Required state and invalid state are exposed after submit.               | Local demo should avoid browser validation bubbles and use ars-ui Form/Checkbox state.     |
| Invalid/error     | Required form failure marks checkbox invalid.                       | Invalid styling and message are visible.                | `aria-invalid` appears after submit; focus moves to invalid checkbox.    | Local error part must drive `aria-describedby` and `aria-errormessage` only when rendered. |
| Focus-visible     | Keyboard/programmatic focus shows visible focus indication.         | Browser focus ring is visible.                          | Focus remains on the checkbox.                                           | ars-ui must expose `data-ars-focus-visible` for styling and tests.                         |
| Hover/pressed     | Styled reference examples show interactive visual affordance.       | Control appearance changes on interaction.              | Semantics stay stable.                                                   | Local widgets/E2E must assert computed visual feedback.                                    |
| Submit            | Required unchecked submit blocks valid submission.                  | Error state appears; form does not submit successfully. | Focus moves to invalid control.                                          | Local Form demo should set visible status and field errors without browser bubble UI.      |
| Reset             | Native reset returns uncontrolled fields to defaults.               | Checkbox returns to initial checked state.              | Form state clears validation/status.                                     | ars-ui Form reset plus Checkbox reset must be demonstrated.                                |

## I18n Mapping

| User-facing text or locale-sensitive behavior | Source                                                                     | Locale/direction cases | Tests or evidence              | Status    | Notes                                                                                        |
| --------------------------------------------- | -------------------------------------------------------------------------- | ---------------------- | ------------------------------ | --------- | -------------------------------------------------------------------------------------------- |
| Label text                                    | Consumer children / widget `Translate`                                     | en-US, pt-BR widgets   | Widget smoke/local screenshots | Supported | Adapter does not own label prose.                                                            |
| Description text                              | Consumer-provided `description` / widget `Translate`                       | en-US, pt-BR widgets   | SSR attrs plus widget smoke    | Supported | Adapter owns relationship attrs, not prose.                                                  |
| Error message                                 | Consumer-provided `error_message` or `ars_forms::validation::Error` source | en-US, pt-BR widgets   | SSR/Form integration tests     | Supported | Checkbox should consume errors for invalid state; visible message remains consumer-rendered. |
| Status announcement                           | Form adapter status message / widget `Translate`                           | en-US, pt-BR widgets   | Form tests and widget smoke    | Supported | Browser-native validation prose is not used in local demo.                                   |

## Accessibility Mapping

| Accessibility axis             | Required DOM/behavior                                                       | Adapter/core source                                     | Tests or evidence | Status    | Notes                                                                     |
| ------------------------------ | --------------------------------------------------------------------------- | ------------------------------------------------------- | ----------------- | --------- | ------------------------------------------------------------------------- |
| Accessible name                | Checkbox control is labelled by visible label.                              | `checkbox::Api::label_attrs` and `control_attrs`        | SSR tests         | Supported | `Control` owns role; label points at hidden input only when safe.         |
| Description/error relationship | Description first, error second; errormessage only when error part renders. | Checkbox core `has_description` and `has_error_message` | SSR + wasm tests  | Supported | Core now avoids dangling error references when invalid content is absent. |
| Keyboard/focus path            | Space toggles, disabled/readonly block mutation, focus-visible is stylable. | Core events plus adapter event mapping                  | wasm + E2E        | Supported | Hidden input must not become the focus target.                            |
| Live region                    | Error message announces politely/alert depending component contract.        | Checkbox error part attrs                               | SSR + axe         | Supported | Match spec after core adjustment.                                         |
| Axe states                     | Initial, toggled, invalid, reset states axe-clean.                          | E2E harness                                             | E2E/browser proof | Supported | Must run after state changes, not only initial.                           |

## Ars Contract Mapping

| Axis                  | Status    | API/contract stance | Agnostic or shared support                    | Adapter support needed              | Widget and E2E support | Tests or evidence | Notes                                                       |
| --------------------- | --------- | ------------------- | --------------------------------------------- | ----------------------------------- | ---------------------- | ----------------- | ----------------------------------------------------------- |
| Basic rendering       | Supported | IdiomaticEquivalent | Checkbox core anatomy                         | Adapter modules                     | Widgets and fixtures   | SSR/E2E           | Custom visual control plus hidden input is acceptable.      |
| Checked/indeterminate | Supported | IdiomaticEquivalent | Checkbox state machine                        | Controlled prop sync                | Widgets and E2E        | wasm/E2E          | Controlled indeterminate must not update optimistically.    |
| Disabled/readonly     | Supported | IdiomaticEquivalent | Checkbox props plus Fieldset context          | Shared field support helper         | Widgets and E2E        | SSR/wasm/E2E      | Readonly is ars-ui extension.                               |
| Invalid or error      | Supported | IdiomaticEquivalent | Checkbox core errors + error-message presence | Shared Form/Fieldset support helper | Widgets and E2E        | New tests         | Avoids dangling `aria-errormessage`.                        |
| Submit                | Supported | IdiomaticEquivalent | Form core + Checkbox hidden input             | Form context integration            | Form demo              | E2E               | Use styled Form/Button flow, not browser bubble.            |
| Reset                 | Supported | SameNativeBehavior  | Checkbox `Reset` event and Form reset         | Adapter reset handling              | Form demo              | wasm/E2E          | Reset returns to defaults and clears visible status/errors. |

Allowed statuses: `Supported`, `RendererSpecific`, `ContractGap`,
`NotApplicable`, `IntentionallyDifferent`.

Allowed API/contract stances: `IdiomaticEquivalent`, `SameNativeBehavior`,
`HigherLevelComposition`, `OutOfScopeApiShape`.

## Parity Audit Loop

Run after the first implementation works and before handoff. Complete at least
three passes and keep looping while any row is `Unknown`, `Unverified`,
`ContractGap`, `AdapterApiGap`, or `WidgetOnlyWorkaround`.

### Pass 1: Reference Outcome Pass

- Date: 2026-06-07
- Findings: React Aria covers labelled checkbox, checked/unchecked, controlled indeterminate, disabled, required validation, focus-visible, submit, and recovery from invalid. Readonly is treated as an ars-ui extension.
- Rows added or split: Split invalid submit, error relationship, focus-visible, hover/pressed visual styling, and reset into separate rows.
- Remaining gaps: None.

### Pass 2: Consumer Reality Pass

- Date: 2026-06-07
- Actual adapter usage checked: Leptos and Dioxus Checkbox adapters, Field helper consumers, E2E fixtures, and widgets.
- Widgets crates checked: all six widgets crates compile; local Leptos widgets page inspected with `playwright-cli`.
- Raw-control or duplicated-policy workarounds: public widgets now use Checkbox, Form, and Button adapters for the form demo; fixture forms use Form adapter for submit/reset evidence.
- Example-owned logic audit:
  - Consumer-owned only: demo labels, descriptions, status copy, and sample submitted values.
  - Component logic found: none after moving form/reset behavior to Form/Checkbox adapter wiring.
  - API gaps opened from examples: Checkbox needed form-error merge and error-message presence tracking; fixed in core/adapters/spec.
- Hardcoded user-facing text found: widgets moved visible Checkbox demo copy to `Translate`; E2E fixture text remains test fixture data.
- Remaining gaps: None.

### Pass 3: I18n, A11y, And Test Proof Pass

- Date: 2026-06-07
- Locale/direction proof: widget text keys added for Checkbox labels, descriptions, errors, buttons, and status messages in en-US and pt-BR.
- Accessibility proof: SSR tests cover labels, invalid/error relationships, no dangling `aria-errormessage`, Fieldset inheritance, and Form validation errors by name; E2E runs axe initially and after reached states.
- Adapter wasm proof: existing wasm tests cover click, Space, controlled indeterminate, focus-visible, disabled, and readonly paths.
- E2E/browser outcome proof: `cargo xtask e2e input --adapter leptos`, `cargo xtask e2e input --adapter dioxus`, `.playwright-cli/local-checkbox-widgets-initial.yml`, and local computed-style eval output.
- Remaining gaps: None.

### Additional Passes

| Pass | Date | Focus | Findings | Remaining gaps |
| ---- | ---- | ----- | -------- | -------------- |
|      |      |       |          |                |

## Final Outcome Matrix

| Reference outcome            | Final status            | API/contract stance | Reference proof                                                | Local proof                                          | Adapter tests                                                                                                                           | E2E/browser proof                                                                  | I18n proof                     | A11y proof                                                | Notes                                                              |
| ---------------------------- | ----------------------- | ------------------- | -------------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------ | --------------------------------------------------------- | ------------------------------------------------------------------ |
| Basic labelled checkbox      | ReferenceOutcomeMatched | IdiomaticEquivalent | `.playwright-cli/reference-checkbox-initial.yml`               | `.playwright-cli/local-checkbox-widgets-initial.yml` | `checkbox_renders_default_anatomy_and_aria`                                                                                             | `cargo xtask e2e input --adapter leptos`, `cargo xtask e2e input --adapter dioxus` | widget `Translate` labels      | SSR label/control assertions                              |                                                                    |
| Checked and unchecked states | ReferenceOutcomeMatched | IdiomaticEquivalent | focus/click eval output                                        | local computed-style eval output                     | `checkbox_renders_checked_indeterminate_and_form_states`                                                                                | E2E state and pointer assertions                                                   | widget `Translate` labels      | `aria-checked` assertions                                 |                                                                    |
| Indeterminate state          | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria controlled docs                                     | local computed-style eval output                     | `checkbox_renders_checked_indeterminate_and_form_states`                                                                                | E2E initial mixed state                                                            | widget `Translate` labels      | `aria-checked="mixed"` assertions                         |                                                                    |
| Disabled state               | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria examples                                            | local computed-style eval output                     | `checkbox_inherits_fieldset_state`                                                                                                      | E2E blocked-state assertions                                                       | widget `Translate` labels      | disabled ARIA assertions                                  |                                                                    |
| Readonly state               | IntentionallyDifferent  | IdiomaticEquivalent | N/A in React Aria primary examples                             | local widget readonly state                          | `checkbox_inherits_fieldset_state`                                                                                                      | E2E readonly blocked-state assertions                                              | widget `Translate` labels      | readonly ARIA assertions                                  | ars-ui extension that remains focusable and blocks mutation.       |
| Required invalid submit      | ReferenceOutcomeMatched | IdiomaticEquivalent | `.playwright-cli/reference-checkbox-after-required-submit.yml` | local computed-style eval output and form status     | `checkbox_inherits_matching_form_validation_errors_by_name`                                                                             | E2E submit status assertion                                                        | widget translated error/status | invalid ARIA assertions                                   | Local demo avoids browser validation bubble by using Form adapter. |
| Error relationship           | ReferenceOutcomeMatched | IdiomaticEquivalent | required-submit eval output                                    | local invalid widget state                           | `checkbox_errors_make_control_invalid_without_dangling_error_reference`, `checkbox_error_message_presence_controls_error_relationships` | E2E invalid anatomy assertions                                                     | widget translated error text   | no dangling `aria-errormessage`; description before error |                                                                    |
| Focus-visible state          | ReferenceOutcomeMatched | IdiomaticEquivalent | focus eval output                                              | wasm/browser adapter tests                           | `checkbox_focus_visible_and_blocked_states_are_reflected`                                                                               | E2E axe after reached states                                                       | Not text-dependent             | focus-visible data attr assertions                        |                                                                    |
| Hover and pressed styling    | ReferenceOutcomeMatched | IdiomaticEquivalent | reference examples                                             | local computed-style eval output                     | visual state covered by attrs tests                                                                                                     | E2E computed dimensions/styles                                                     | Not text-dependent             | semantics stable after pointer                            |                                                                    |
| Form reset                   | ReferenceOutcomeMatched | SameNativeBehavior  | native reset semantics                                         | local form reset status output                       | wasm reset path covered by controlled reset tests                                                                                       | E2E reset status and `aria-checked` restored                                       | widget translated reset status | reset leaves valid checked state                          |                                                                    |

Allowed final statuses: `ReferenceOutcomeMatched`,
`IntentionallyDifferent`, `OutOfScopeWithReason`.

Allowed API/contract stances: `IdiomaticEquivalent`, `SameNativeBehavior`,
`HigherLevelComposition`, `OutOfScopeApiShape`.

## Contract Gaps Before Coding

| Gap                                                                                    | Evidence                                                                     | Required fix                                                                                              | Spec update needed                              |
| -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Checkbox invalid state cannot consume Form validation errors by `name`.                | Field already does this through Form context; Checkbox WIP did not.          | Fixed: added `errors` to Checkbox core and adapter props, merged named Form errors through shared helper. | Yes                                             |
| `aria-errormessage` can be emitted whenever invalid even if no error part is rendered. | Previous core had only `has_description`; invalid directly added error refs. | Fixed: added error-message presence tracking and only reference error message when the part exists.       | Yes                                             |
| Field merge logic is duplicated/private.                                               | Field adapters each owned local merge helpers.                               | Fixed: extracted crate-internal shared helper in both adapters and updated Field + Checkbox to use it.    | No public spec change beyond Checkbox behavior. |

## Implementation Sketch

1. Agnostic/spec changes: add Checkbox `errors`, invalid-from-errors, and error-message presence tracking; update adapter specs.
2. Leptos adapter changes: add `utility::field_support`, update Field, add Checkbox `errors` prop and Form/Fieldset merge.
3. Dioxus adapter changes: mirror Leptos helper and Checkbox integration with plain props.
4. Widget changes: update all six input demos to use translated strings, styled Checkbox states, Form/Button submit/reset flow, and no browser validation bubbles.
5. E2E/browser harness changes: extend input/checkbox harness with computed visual assertions, submit/reset serialization, and axe reached states.

## Verification Plan

- Focused tests:
  - `cargo test -p ars-leptos --test checkbox`
  - `cargo test -p ars-dioxus --test checkbox`
- I18n tests: widget checks plus locale smoke through browser evidence.
- Accessibility tests: SSR semantic assertions, wasm focus/keyboard, E2E axe after reached states.
- E2E command:
  - `cargo xtask e2e input --adapter leptos`
  - `cargo xtask e2e input --adapter dioxus`
- Widget smoke: `cargo check --manifest-path` for all six widgets crates plus `playwright-cli` local snapshot.
- Browser reference/local comparison: reference artifacts above, local artifacts to be added after implementation.
- `cargo xtask lint adapter-parity`
- `cargo xtask spec validate`
- `cargo +nightly fmt --all --check`

## Handoff Update

- Local evidence paths: `.playwright-cli/local-checkbox-widgets-initial.yml`; local computed-style eval output in implementation log.
- Parity audit loop passes completed: 3.
- Final outcome counts: 9 `ReferenceOutcomeMatched`, 1 `IntentionallyDifferent`, 0 `OutOfScopeWithReason`.
- Rows still Unknown/Unverified/ContractGap/AdapterApiGap/WidgetOnlyWorkaround: none.
- Final parity status: outcome-complete for supported Checkbox axes; readonly intentionally differs as an ars-ui extension.
- Final i18n status: complete for public widgets; E2E fixture prose is test fixture data.
- Final accessibility status: complete for SSR relationships, wasm interactions, E2E axe reached states, and no dangling error IDREFs.
- Remaining `NotApplicable` axes: none.
- Remaining `IntentionallyDifferent` axes: readonly state is supported by ars-ui even though it is not a primary React Aria Checkbox example axis.
- Remaining risks: browser screenshots are not checked in; reproducible snapshots/evals are under `.playwright-cli/`.
