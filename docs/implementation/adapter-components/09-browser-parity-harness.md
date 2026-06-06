# Browser Parity Harness

This workflow turns counterpart comparison into repeatable evidence. Use it for
every visible adapter component before claiming outcome parity.

The first pass against the reference implementation happens before coding and
is recorded in the implementation sketch described in
[10-reference-exploration-sketch.md](10-reference-exploration-sketch.md). The
later local-vs-reference pass updates the same sketch and the PR body with
local evidence.

## Counterpart Sessions

Use `playwright-cli` with separate sessions for the reference page and local
widgets page:

```bash
playwright-cli -s=reference open <counterpart-url>
playwright-cli -s=local open http://localhost:<port>/
```

The in-app browser is fine for quick orientation, but PR evidence must be
reproducible with `playwright-cli` commands or a checked-in browser harness.

Store artifacts only under `.playwright-cli/` or `/tmp/`. Do not add snapshots,
screenshots, traces, or videos to the repo root.

## Required Evidence Loop

For each supported counterpart axis:

1. Capture the reference state.
2. Drive the same local state.
3. Compare behavior, visible feedback, and accessibility state.
4. Record the artifact path or harness test that proves the comparison.
5. Update the sketch matrix row with the proof and final status.

Use snapshots after page load and after every meaningful state transition:

```bash
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-checkbox-invalid.yml
playwright-cli -s=local snapshot --filename=.playwright-cli/local-checkbox-invalid.yml
```

Use screenshots only when geometry, spacing, icon shape, or visual alignment is
the thing being reviewed:

```bash
playwright-cli -s=reference screenshot --filename=/tmp/reference-checkbox-invalid.png
playwright-cli -s=local screenshot --filename=/tmp/local-checkbox-invalid.png
```

## Computed Checks

Do not rely on screenshots alone. Use `eval` for measurable state:

```bash
playwright-cli -s=local eval "el => {
  const style = getComputedStyle(el);
  const rect = el.getBoundingClientRect();
  return {
    ariaChecked: el.getAttribute('aria-checked'),
    ariaInvalid: el.getAttribute('aria-invalid'),
    state: el.getAttribute('data-ars-state'),
    color: style.color,
    backgroundColor: style.backgroundColor,
    borderColor: style.borderColor,
    cursor: style.cursor,
    width: rect.width,
    height: rect.height
  };
}" e5
```

Check at least the state attrs, ARIA attrs, computed colors, cursor/opacity when
disabled, visible dimensions, and whether controls keep stable dimensions after
interaction.

## Forms And Validation

For form-participating components, the browser review must exercise both submit
and reset paths. Avoid treating browser-native validation popups as sufficient
visual proof unless the chosen parity target intentionally uses them.

Use `eval` to inspect serialized form values and hidden inputs:

```bash
playwright-cli -s=local eval "() => {
  const form = document.querySelector('#component-demo-form');
  return {
    values: Array.from(new FormData(form).entries()),
    hiddenInputs: Array.from(form.querySelectorAll('input[type=\"hidden\"], input.ars-sr-input'))
      .map(input => ({
        name: input.name,
        value: input.value,
        checked: input.checked,
        required: input.required,
        disabled: input.disabled
      }))
  };
}"
```

Submit and reset controls should use ars-ui components when the demo is proving
ars-ui component integration, not raw browser buttons, unless the component
under review is the raw form primitive itself.

## Console And Network

Before presenting work, inspect the browser console after representative
interactions:

```bash
playwright-cli -s=local console
playwright-cli -s=local console warning
playwright-cli -s=local network
```

Reactive-context warnings, hydration warnings, missing asset failures, and
uncaught exceptions are user-visible regressions.

## Outcome Matrix

Record the result in the implementation sketch and PR body:

| Reference outcome                  | Reference evidence                    | Local evidence                    | Tests/E2E                      | I18n proof                       | A11y proof                    | Status                  | Notes                                 |
| ---------------------------------- | ------------------------------------- | --------------------------------- | ------------------------------ | -------------------------------- | ----------------------------- | ----------------------- | ------------------------------------- |
| Basic labelled control             | `.playwright-cli/reference-basic.yml` | `.playwright-cli/local-basic.yml` | `basic_pointer_and_keyboard`   | `pt-BR local snapshot`           | `axe basic`                   | ReferenceOutcomeMatched | Matches outcome, not API shape        |
| Invalid message shown after submit | `/tmp/reference-invalid.png`          | `/tmp/local-invalid.png`          | `invalid_visual_and_axe_clean` | browser-native localized message | `aria-errormessage` assertion | ReferenceOutcomeMatched | Custom error UI replaces native popup |
| Loading state                      | NotApplicable                         | NotApplicable                     | NotApplicable                  | NotApplicable                    | NotApplicable                 | OutOfScopeWithReason    | Component has no loading state        |

Use the final statuses from
[12-parity-audit-loop.md](12-parity-audit-loop.md):
`ReferenceOutcomeMatched`, `IntentionallyDifferent`, or
`OutOfScopeWithReason`. Do not leave a counterpart feature unclassified, and do
not use `outcome-complete` while any row is `Unknown`, `Unverified`,
`ContractGap`, `AdapterApiGap`, or `WidgetOnlyWorkaround`.
