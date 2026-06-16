# Field/Form Adapter Reference Exploration Sketch

## Task

- Issues: #332, #423
- Component: Field, Fieldset, Form
- Category: utility
- Adapters in scope: Leptos, Dioxus
- Specs read: `spec/leptos-components/utility/field.md`, `spec/dioxus-components/utility/field.md`, `spec/leptos-components/utility/form.md`, `spec/dioxus-components/utility/form.md`, `spec/components/utility/field.md`, `spec/components/utility/fieldset.md`, `spec/components/utility/form.md`
- Date: 2026-06-04

## Reference Sources

- Primary counterpart: React Aria Form
- Primary URL: <https://react-aria.adobe.com/Form>
- Fallback counterparts inspected: NotApplicable
- Reason for fallback or N/A: React Aria has the component and is the primary adapter reference.

## Playwright Exploration Commands

```bash
playwright-cli -s=reference open https://react-aria.adobe.com/Form
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-form-initial.yml
playwright-cli -s=reference click e275
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-form-empty-submit.yml
playwright-cli -s=reference screenshot --filename=/tmp/reference-form-empty-submit.png
playwright-cli -s=reference run-code "async page => { /* inspect email validity states */ }"
playwright-cli -s=reference run-code "async page => { /* inspect reset behavior */ }"
playwright-cli -s=reference run-code "async page => { /* inventory all React Aria Form examples */ }"
```

## Reference Evidence

| State or outcome               | Command or action                                                              | Artifact                                                                                  | Notes                                                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Page baseline                  | `snapshot --filename=.playwright-cli/reference-form-initial.yml`               | `.playwright-cli/reference-form-initial.yml`                                              | Shows five React Aria Form examples: basic native validation, submit serialization, custom validation, server validation, and form-level invalid handling. |
| Empty required submit          | Click first example Submit, then snapshot                                      | `.playwright-cli/reference-form-empty-submit.yml`, `/tmp/reference-form-empty-submit.png` | Required fields become invalid, errors are linked through `aria-describedby`, and messages come from browser/native validity.                              |
| Email missing at-sign          | Fill name `Test`, email `e`, click Submit, inspect with `run-code`             | `.playwright-cli/reference-form-email-missing-at.yml`                                     | Email input has `aria-invalid="true"`, `validity.typeMismatch=true`, and message `Inclua um "@" no endereço de e-mail...`.                                 |
| Email missing domain           | Fill name `Test`, email `email@`, click Submit, inspect with `run-code`        | command output                                                                            | Email input has `aria-invalid="true"`, `validity.typeMismatch=true`, and message `Insira uma parte depois de "@". "email@" está incompleto.`               |
| Email valid enough for browser | Fill name `Test`, email `email@example`, click Submit, inspect with `run-code` | command output                                                                            | Browser treats `email@example` as valid for `type=email`; React Aria follows native validity.                                                              |
| Reset after invalid            | Fill invalid email, submit, click Reset, inspect with `run-code`               | command output                                                                            | Reset clears values and removes `aria-invalid` / error relationship while native `validity.valid` remains false for empty required fields.                 |
| Server validation              | Inventory form index 3                                                         | command output                                                                            | `validationErrors` renders server error text, sets `aria-invalid="true"` and `data-invalid="true"`, and links text through `aria-describedby`.             |

## Observed Reference Outcomes

| Axis                     | Reference behavior                                                             | User-visible outcome                                            | Accessibility outcome                                                                         | Notes                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------ | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Basic rendering          | Form renders labeled TextFields, submit and reset buttons.                     | Quiet vertical form layout with labels above controls.          | Inputs use label relationships and stable IDs.                                                | The simplest example sets the widget baseline.                                                               |
| Required empty submit    | Native validation blocks first form submission and renders per-field messages. | Required errors appear under both fields.                       | Invalid inputs get `aria-invalid="true"` and `aria-describedby` pointing at error text.       | Browser locale controls message text.                                                                        |
| Email malformed value    | Native `type=email` subreason messages are surfaced.                           | Missing at-sign and missing domain produce different messages.  | Same invalid/error relationship as required errors.                                           | `email@example` is valid by browser rules.                                                                   |
| Reset                    | Reset clears invalid display and values.                                       | Error text disappears and fields return to defaults.            | `aria-invalid` is removed after reset.                                                        | Required empty values remain natively invalid but are not displayed as errors until validation is triggered. |
| Submit serialization     | Secondary example serializes `FormData` in `onSubmit`.                         | Submitted values are available to app logic without navigation. | Native form semantics are preserved.                                                          | Adapter can expose event composition or allow consumer handlers.                                             |
| Custom validation        | `validate` returns custom text such as `Nice try.`.                            | Custom text appears as field error.                             | Field is marked invalid and error text is linked.                                             | Higher-level validation API is needed for equivalent custom rules.                                           |
| Server validation        | `validationErrors={{ username: ... }}` drives field state.                     | Server error is shown immediately.                              | Input has `aria-invalid="true"`, root has `data-invalid="true"`, and error text is described. | This is specified by ars-ui `Form.validation_errors` but missing in current adapters.                        |
| Form-level invalid alert | `onInvalid` can prevent default focus behavior and show form alert.            | A summary alert appears before fields.                          | Alert can receive focus.                                                                      | This is higher-level composition around `Form`, not a required low-level part.                               |

## Ars Contract Mapping

| Axis                            | Status        | Agnostic or shared support                                                                                                                                         | Adapter support needed                                                                                                         | Widget and E2E support                                                                                                                     | Tests or evidence                                                                                                          | Notes                                                                                    |
| ------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Basic Field/Form structure      | Supported     | `field::Machine`, `fieldset::Machine`, and `form::Machine` derive IDs and attrs.                                                                                   | Both adapters render root/subparts.                                                                                            | All six widgets show Field/Form.                                                                                                           | Existing adapter tests and E2E should remain.                                                                              | Keep.                                                                                    |
| Required/native validity attrs  | Supported     | `Field` has `required`, but emits only ARIA required.                                                                                                              | `Field::Input` should also set native `required` for native form controls.                                                     | Widgets should use `Field::Input` instead of raw input once event props work.                                                              | Add adapter tests for native `required`.                                                                                   | Needed for React Aria native-validation parity.                                          |
| Disabled/readonly native attrs  | Supported     | `Field` has disabled/readonly state.                                                                                                                               | `Field::Input` should set native `disabled` and `readonly` in addition to ARIA attrs.                                          | Widgets can rely on native behavior.                                                                                                       | Add adapter tests.                                                                                                         | Existing hidden participant guidance says native disabled matters.                       |
| Field error message from errors | ContractGap   | Core `Field` supports `SetErrors` / `ClearErrors`, but adapter public API cannot set errors.                                                                       | Add error-vector/message props and sync them into the field machine.                                                           | Widgets and E2E should drive `ErrorMessage`, not sibling alerts.                                                                           | Add tests for error text visibility and `aria-describedby` / `aria-errormessage`.                                          | Current `invalid: bool` is not enough.                                                   |
| Server validation errors        | ContractGap   | Core `Form` has `validation_errors`, but adapters do not expose it or propagate errors to fields.                                                                  | Add `validation_errors` prop to both Form adapters, and provide form context so fields can consume errors by name.             | Widgets should show server-error example.                                                                                                  | Add Form tests plus Field/Form integration tests.                                                                          | Spec requires this prop.                                                                 |
| Email subreason messages        | Supported     | Native browser validity provides localized subreason messages for `type=email`; demo validation mirrors the distinct outcomes through component-owned error parts. | `Field::Input` exposes `on_value_input` in both adapters so consumers can run validation handlers without raw native controls. | Dioxus Tailwind widget demonstrates empty, missing at-sign, missing domain, and valid states through `FieldInput` and `FieldErrorMessage`. | `field_input_emits_value_input_callback`, Dioxus wasm compile smoke, local browser validation snapshots.                   | Exact browser text is locale-specific; tests assert semantics and localized widget text. |
| Submit/reset lifecycle          | Supported     | Core `Form` has `Submit` / `Reset`.                                                                                                                                | Both adapters dispatch core submit/reset events and expose `on_submit` / `on_reset` callbacks.                                 | Dioxus Tailwind widget uses `Form` callbacks plus ars-ui `Button` submit/reset controls.                                                   | `form_submit_and_reset_callbacks_fire_and_block_native_submit`, Dioxus wasm compile smoke, local browser reset assertions. | Native navigation is prevented when an `on_submit` callback is present.                  |
| Custom validation               | NotApplicable | `ars-forms` has validators, but low-level `Field`/`Form` adapters do not expose a full validator registration API yet.                                             | Document as higher-level form integration unless current task expands scope.                                                   | Not required for this low-level pass.                                                                                                      | N/A.                                                                                                                       | Server errors and explicit field errors cover current adapter spec.                      |
| Form-level invalid alert        | NotApplicable | No low-level Form part for summary alert.                                                                                                                          | Consumers can compose alerts around Form.                                                                                      | Not required.                                                                                                                              | N/A.                                                                                                                       | React Aria demonstrates composition via `onInvalid`.                                     |

Allowed statuses: `Supported`, `RendererSpecific`, `ContractGap`,
`NotApplicable`, `IntentionallyDifferent`.

## Contract Gaps Before Coding

| Gap                                                                         | Evidence                                                                                                       | Required fix                                                                                                                                 | Spec update needed                                                |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Field adapters cannot drive core `SetErrors` / `ClearErrors`.               | Current Leptos/Dioxus `Field` props expose `invalid: bool` only; React Aria error text is linked to the input. | Add public error-message/error-vector props and sync them to `field::Machine`; render `ErrorMessage` from the machine relationship.          | Update adapter specs if prop names differ from existing examples. |
| Form adapters do not expose `validation_errors`.                            | Specs require full parity with `validation_errors`; current adapter props omit it.                             | Add `validation_errors` props to Leptos and Dioxus Form and feed `form::Props::validation_errors`.                                           | No, unless API shape differs from spec.                           |
| Form errors are not propagated to descendant fields.                        | React Aria `validationErrors` marks the matching field invalid and links error text.                           | Provide form context with typed validation errors and have `Field` merge errors by input `name` or explicit field name.                      | Clarify precedence if needed.                                     |
| Form adapters do not dispatch submit/reset machine events.                  | Core Form has lifecycle events; adapters previously only rendered `<form>`.                                    | Fixed: both adapters dispatch `Submit` / `Reset` and expose `on_submit` / `on_reset`.                                                        | Yes: adapter specs document the explicit callbacks.               |
| Dioxus event props did not work for `Form` / `Input` in the widget attempt. | Earlier widget compile failed when using `onsubmit` / `oninput` on custom components.                          | Fixed: both adapters expose explicit `on_value_input`, `on_submit`, and `on_reset` props so component users do not need raw native controls. | Yes: adapter specs document the explicit callbacks.               |

## Implementation Sketch

1. Agnostic/spec changes: No new core changes required. Core already has field errors, form validation errors, and form lifecycle events. Adapter specs now document `Input.on_value_input`, `Form.on_submit`, and `Form.on_reset`.
2. Leptos adapter changes: Add `errors` to `Field`, native `required`/`disabled`/`readonly` attrs on `Input`, `validation_errors` to `Form`, form-context error propagation by field name, `Input.on_value_input`, and `Form.on_submit` / `Form.on_reset`.
3. Dioxus adapter changes: Same as Leptos, using `EventHandler<T>` for explicit event props.
4. Widget changes: Replace demo-local error paragraph with `Field::ErrorMessage`; keep validation messages in component-facing state until a higher-level validator API exists. Dioxus Tailwind interactive demo now uses `FieldInput`, `Form` callbacks, and ars-ui `Button` submit/reset controls.
5. E2E/browser harness changes: Add cases for required empty, malformed email, server error, reset clears error relationship, and native attrs on the actual input.

## Verification Plan

- Focused tests: `cargo test -p ars-leptos --test field --test form`, `cargo test -p ars-dioxus --test field --test form`
- E2E command: focused utility field/form harness after implementation
- Widget smoke: Dioxus Tailwind local page with empty, missing at, missing domain, valid, and reset cases
- Browser reference/local comparison: `.playwright-cli/reference-form-*.yml` and local snapshots
- `cargo xtask lint adapter-parity`
- `cargo xclippy`

## Parity Audit Loop

### Pass 1: Reference Outcome Pass

- Date: 2026-06-04
- Findings: The matrix needed separate rows for distinct email validation outcomes, submit lifecycle, reset lifecycle, server validation, custom validation, and form-level alert composition.
- Rows added or split: Email subreason messages and submit/reset lifecycle were promoted from generic validation/form rows to explicit supported outcomes.
- Remaining gaps: Custom validation API and form-level invalid summary are not low-level primitive outcomes for this pass.

### Pass 2: Consumer Reality Pass

- Date: 2026-06-04
- Actual adapter usage checked: `examples/widgets-dioxus-tailwind/src/categories/utility.rs` interactive Field/Form demo.
- Widgets crates checked: all six widgets crates already render Field/Form sections; the only raw-control workaround for interactive validation was in Dioxus Tailwind.
- Raw-control or duplicated-policy workarounds: Removed for supported outcomes. The Dioxus Tailwind demo now uses `FieldInput`, `FieldErrorMessage`, `Form.on_submit`, `Form.on_reset`, and ars-ui `Button` submit/reset controls.
- Hardcoded user-facing text found: No new hardcoded component text. Field/Form labels, placeholders, validation messages, status text, and buttons are translated through `UtilityText`.
- Remaining gaps: None for supported Field/Form outcomes.

### Pass 3: I18n, A11y, And Test Proof Pass

- Date: 2026-06-04
- Locale/direction proof: Local browser pt-BR pass verifies Field/Form visible text and validation messages are localized. Native browser validity messages remain browser-owned; widget validation messages are `Translate`-owned.
- Accessibility proof: Field input attrs come from the core Field API; local browser assertions verify `aria-invalid`, `aria-describedby`, and `aria-errormessage` update for invalid and reset states.
- Adapter wasm proof: Leptos and Dioxus wasm tests cover real DOM-mounted field/form/fieldset attrs plus `Input.on_value_input` and `Form.on_submit` / `Form.on_reset` browser events.
- E2E/browser outcome proof: focused SSR tests cover field/form semantic markup; local browser evidence covers the public widget flow and final parity matrix rows.
- Remaining gaps: No `Unknown`, `Unverified`, `ContractGap`, `AdapterApiGap`, or `WidgetOnlyWorkaround` rows for supported outcomes.

## Final Outcome Matrix

| Reference outcome                            | Final status            | API/contract stance    | Reference proof                                                                           | Local proof                                                                                 | Adapter tests                                                                             | E2E/browser proof                     | I18n proof                                                    | A11y proof                                                               | Notes                                                                                                                                                    |
| -------------------------------------------- | ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Basic labelled Field/Form structure          | ReferenceOutcomeMatched | IdiomaticEquivalent    | `.playwright-cli/reference-form-initial.yml`                                              | `.playwright-cli/local-field-form-initial.yml`, `.playwright-cli/local-field-form-ptbr.yml` | `field_renders_root_label_input_and_messages`, `form_renders_root_and_status_region`      | local widget snapshot                 | pt-BR widget text verification                                | label/input relationship assertions                                      |                                                                                                                                                          |
| Required empty submit shows field error      | ReferenceOutcomeMatched | SameNativeBehavior     | `.playwright-cli/reference-form-empty-submit.yml`, `/tmp/reference-form-empty-submit.png` | `.playwright-cli/local-field-form-empty-submit.yml`                                         | `form_validation_errors_drive_matching_field_by_name`                                     | local browser empty-submit assertions | `UtilityText::DemoEmailRequired`                              | `aria-invalid`, `aria-describedby`, `aria-errormessage` assertions       |                                                                                                                                                          |
| Email missing at-sign has distinct message   | ReferenceOutcomeMatched | IdiomaticEquivalent    | `.playwright-cli/reference-form-email-missing-at.yml`                                     | `.playwright-cli/local-field-form-missing-at.yml`                                           | `field_input_emits_value_input_callback`                                                  | local browser interaction             | `UtilityText::DemoEmailMissingAt`                             | field error relationship from core attrs                                 |                                                                                                                                                          |
| Email missing domain has distinct message    | ReferenceOutcomeMatched | IdiomaticEquivalent    | reference `run-code` output                                                               | `.playwright-cli/local-field-form-missing-domain.yml`                                       | `field_input_emits_value_input_callback`                                                  | local browser interaction             | `UtilityText::DemoEmailMissingDomain` / `DemoEmailMissingDot` | field error relationship from core attrs                                 |                                                                                                                                                          |
| Valid email clears invalid state             | ReferenceOutcomeMatched | IdiomaticEquivalent    | reference `run-code` output                                                               | `.playwright-cli/local-field-form-valid.yml`                                                | `field_input_emits_value_input_callback`                                                  | local browser interaction             | `UtilityText::ValidationPassed`                               | no invalid attr and no error relationship                                |                                                                                                                                                          |
| Reset clears displayed errors and values     | ReferenceOutcomeMatched | IdiomaticEquivalent    | reference `run-code` output                                                               | `.playwright-cli/local-field-form-reset.yml`                                                | `form_submit_and_reset_callbacks_fire_and_block_native_submit`                            | local browser reset interaction       | `UtilityText::ReadyToSubmit`                                  | reset removes invalid relationship; hidden error node is `display: none` |                                                                                                                                                          |
| Submit values available to app logic         | ReferenceOutcomeMatched | IdiomaticEquivalent    | React Aria submit serialization example                                                   | Dioxus Tailwind `Form.on_submit` state update                                               | `form_submit_and_reset_callbacks_fire_and_block_native_submit`, Dioxus wasm compile smoke | local browser submit interaction      | translated status text                                        | core `Submit` event before callback                                      | Rich FormData payload remains future higher-level API.                                                                                                   |
| Server validation errors mark matching field | ReferenceOutcomeMatched | IdiomaticEquivalent    | React Aria server validation example                                                      | adapter SSR output                                                                          | `form_validation_errors_drive_matching_field_by_name` in both adapters                    | covered by adapter semantic tests     | consumer/server-provided localized text                       | `aria-invalid` and `aria-errormessage` on matching field only            |                                                                                                                                                          |
| Custom validation registration API           | OutOfScopeWithReason    | HigherLevelComposition | React Aria custom validation example                                                      | NotApplicable                                                                               | NotApplicable                                                                             | NotApplicable                         | consumer-provided text                                        | consumer-composed field errors                                           | Low-level Field/Form adapters expose explicit typed field and form validation errors; validator registration belongs to a higher-level form integration. |
| Form-level invalid summary alert             | OutOfScopeWithReason    | HigherLevelComposition | React Aria form-level invalid handling example                                            | NotApplicable                                                                               | NotApplicable                                                                             | NotApplicable                         | consumer-provided text                                        | consumer-owned alert semantics                                           | Composed by consumers around `Form`; not a required low-level `Form` part.                                                                               |

## Handoff Update

- Local evidence paths: `.playwright-cli/local-field-form-initial.yml`, `.playwright-cli/local-field-form-ptbr.yml`, `.playwright-cli/local-field-form-empty-submit.yml`, `.playwright-cli/local-field-form-missing-at.yml`, `.playwright-cli/local-field-form-missing-domain.yml`, `.playwright-cli/local-field-form-valid.yml`, `.playwright-cli/local-field-form-reset.yml`, `.playwright-cli/console-2026-06-04T22-13-29-121Z.log`
- Parity audit loop passes completed: 3
- Final outcome counts: 8 `ReferenceOutcomeMatched`, 0 `IntentionallyDifferent`, 2 `OutOfScopeWithReason`
- API/contract stance counts: 7 `IdiomaticEquivalent`, 1 `SameNativeBehavior`, 2 `HigherLevelComposition`, 0 `OutOfScopeApiShape`
- Rows still Unknown/Unverified/ContractGap/AdapterApiGap/WidgetOnlyWorkaround: none for supported outcomes
- Local browser assertions:
  - empty submit: `aria-invalid="true"`, `aria-describedby="dioxus-tw-email-field-description dioxus-tw-email-field-error-message"`, `aria-errormessage="dioxus-tw-email-field-error-message"`, visible `ErrorMessage` text `Digite um endereço de e-mail.`
  - missing at-sign: visible `ErrorMessage` and status text `Inclua um @ no endereço de e-mail.`
  - missing domain: visible `ErrorMessage` and status text `Digite a parte depois de @.`
  - valid email: invalid attr absent, `aria-describedby="dioxus-tw-email-field-description"`, error text empty, status text `Validação aprovada.`
  - reset: name and email values cleared, invalid attr and `aria-errormessage` absent, `aria-describedby="dioxus-tw-email-field-description"`, hidden error node has `hidden=true`, `display: none`, and zero layout, status text `Pronto para enviar`.
- Final parity status: outcome-complete for the low-level Field/Form adapter scope
- Remaining `NotApplicable` axes: Custom validation API, form-level invalid summary alert
- Remaining `IntentionallyDifferent` axes:
- Remaining risks: Rich `FormData` submit payloads and first-class validator registration remain higher-level form integration concerns; the low-level adapter callbacks intentionally expose normalized lifecycle/value hooks only.

## Leptos Validation Regression Addendum

- Date: 2026-06-04
- Browser regression: Leptos Tailwind `Field/Form` showed `admin@email.com` with the stale required error after submit because native form navigation reached `/account?name=&email=admin%40email.com`.
- Root causes:
  - Leptos `Field` / `Form` adapter props were not fully reactive (`invalid`, `errors`, `validation_errors`, and `Input.value` could be fixed at mount).
  - Leptos `Input` rendered relationship attrs from a static attr map, so `aria-invalid` / `aria-errormessage` could remain stale after the field became valid.
  - The Leptos machine hook updated Leptos signals while its `StoredValue<Service<_>>` was mutably borrowed, which caused runtime reactive borrow panics in the full browser app.
- Fixes:
  - `Field` and `Form` now sync reactive prop signals into their machines.
  - `Input.value`, required/disabled/readonly native attrs, and invalid/error relationship attrs now render reactively from the live field API.
  - `use_machine_with_reactive_props` and `send` now extract state/context while the service is borrowed, then update Leptos signals after the borrow ends.
  - Leptos Tailwind Field/Form now demonstrates validation through the adapter components instead of static invalid styling.
- Browser proof:
  - Clean Playwright session on `http://127.0.0.1:5302/`: empty submit stays on `/` with `aria-invalid="true"` and `Enter an email address.`; filling `admin@email.com` clears the invalid attr and error text; valid submit stays on `/` with `Validation passed.`; reset clears value and returns `Ready to submit`.
  - Clean console after load/workflow: 0 errors. Remaining warnings are existing Leptos locale-tracking warnings in the examples app, not Field/Form validation panics.
- Test proof:
  - `field_reactive_errors_update_invalid_relationship`
  - `form_submit_button_click_fires_submit_callback_without_navigation`
  - Focused Leptos wasm and SSR Field/Form suites pass after the fix.

## Leptos Error Message Visibility Regression Addendum

- Date: 2026-06-04
- Browser regression: invalid email showed the validation text only in the form status region while the field error node below the input remained hidden.
- Root cause: Leptos `Input` relationship attrs were reactive, but `FieldErrorMessage` still rendered `hidden` from a one-time `error_message_attrs()` snapshot. That left the input pointing at an error node that had `hidden=""` and `display: none`.
- Fix: `FieldErrorMessage` now renders `HtmlAttr::Hidden` as a reactive boolean attr sourced from the live field API.
- Browser proof: fresh Leptos Tailwind server on `http://127.0.0.1:5303/` with email `d` after submit produced `aria-invalid="true"`, `aria-errormessage="leptos-tw-email-field-error-message"`, visible field error text `Include an @ in the email address.`, `hidden=null`, `display=block`, and non-zero height. Filling `admin@email.com` removed the invalid/error relationship and hid the field error again.
- Test proof: `field_reactive_errors_update_invalid_relationship` now asserts the field error message shows while invalid and hides when valid.

## Tailwind Example Scope Addendum

- Date: 2026-06-04
- Decision: Leptos and Dioxus Tailwind `Field/Form` examples should not carry custom email validation policy, per-input class switching, or derived validation message logic. Those heavier behaviors belong to the future `TextField` adapter PR.
- Fix: both Tailwind examples now keep the `Field/Form/Fieldset` demo as a low-level relationship showcase with labels, descriptions, required inputs, submit/reset controls, and a status region. The examples no longer implement demo-local email validation or dynamic invalid styling.
- Follow-up: future `TextField` should own the React Aria-style text input outcomes: value tracking, native constraint handling, validation message selection, invalid data attrs/styling hooks, and default error display wiring.

## Retrofit Audit Addendum

- Date: 2026-06-16
- Audit issue: #730
- Prior implementation context: #332 and #423
- Current workflow baseline: Checkbox-era adapter delivery workflow, including compound-part `class`/`style` support, Dioxus `GlobalAttributes`, no duplicated semantic helpers in adapter files, and browser-backed parity evidence.

### Fresh Findings

| Area                             | Finding                                                                                                                                            | Fix                                                                                                                                                                                     |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native validity classification   | Leptos and Dioxus `Form` adapters duplicated native validity classification and form-error map merging.                                            | Moved reusable `NativeValidity`, `NativeInputType`, and `merge_error_map` semantics to `ars_forms::validation`; adapters now only extract browser DOM facts.                            |
| Compound part styling            | `Field`, `Fieldset`, and `Form` subparts predated the Checkbox adapter surface and did not consistently expose consumer `class`/`style` / globals. | Leptos `Label`, `Input`, `Description`, `ErrorMessage`, `Legend`, `Content`, and `StatusRegion` now accept additive `class` and `style`; Dioxus subparts now extend `GlobalAttributes`. |
| Form root styling parity         | Dioxus `Form` root accepted global `class`/`style`; Leptos `Form` root only accepted `class`.                                                      | Added Leptos `Form.style` using the shared reactive style helper.                                                                                                                       |
| Dioxus Fieldset browser evidence | Dioxus had initial inherited-state browser coverage but lacked parity tests for fieldset error inheritance and reactive inherited-state updates.   | Added Dioxus wasm tests matching the Leptos evidence rows.                                                                                                                              |
| Dioxus form context              | A wasm compile surfaced an unused `reset_generation` field stored in form context even though reset generation is only used inside `Form`.         | Removed the unused context field and kept the signal local to the adapter root.                                                                                                         |
| Spec drift                       | Adapter specs and the forms foundation still described older public API/error shapes in several places.                                            | Updated adapter API snippets, attr-merge notes, and `Form.validation_errors` to structured `ars_forms::validation::Error`.                                                              |

### Final Retrofit Outcome Matrix

| Outcome                                                                                  | Status                 | Evidence                                                                                                                                                                                |
| ---------------------------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Field/Form/Fieldset SSR structure and root styling parity                                | Supported              | `cargo test -p ars-leptos --features ssr --test field --test fieldset --test form`; `cargo test -p ars-dioxus --test field --test fieldset --test form`                                 |
| Field and Fieldset compound part consumer styling/globals                                | Supported              | New SSR tests: `field_parts_accept_consumer_class_and_style`, `fieldset_parts_accept_consumer_class_and_style` in both adapters                                                         |
| Native validity and error-map semantics kept outside renderer glue                       | Supported              | `cargo test -p ars-forms --lib native`                                                                                                                                                  |
| Field reactive errors, input relationships, and controlled input values                  | Supported              | Leptos wasm `test_field_wasm` 4 tests; Dioxus wasm `test_field_wasm` 5 tests                                                                                                            |
| Fieldset inherited disabled/readonly/invalid/error state                                 | Supported              | Leptos wasm `test_fieldset_wasm` 4 tests; Dioxus wasm `test_fieldset_wasm` 4 tests                                                                                                      |
| Form validation behavior, native invalid submit, server errors, reset, and status region | Supported              | Leptos wasm `test_form_wasm` 9 tests; Dioxus wasm `test_form_wasm` 10 tests                                                                                                             |
| Dioxus hook-order probe                                                                  | Supported              | `rg 'unwrap*or_else\(\|\| use*                                                                                                                                                          | map*or_else\([^\n]\*use*' crates/ars-dioxus/src/utility/{field,fieldset,form,field_support}.rs` returned no matches |
| Tailwind example scope after addendum                                                    | IntentionallyDifferent | Existing 2026-06-04 addendum remains current: Field/Form/Fieldset examples stay low-level relationship showcases; richer text-input validation policy remains future `TextField` scope. |

### Intentionally Different Or Out Of Scope

- Custom validator registration remains higher-level form integration scope, not a low-level `Field`/`Form` adapter requirement.
- Form-level invalid summary alert remains consumer composition around `Form`, not a required low-level `Form` part.
- Rich `FormData` submit payloads remain higher-level API scope; the low-level adapters expose normalized lifecycle callbacks.
