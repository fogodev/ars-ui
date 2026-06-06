# Component Adapter Reference Exploration Sketch

## Task

- Issues:
- Component:
- Category:
- Adapters in scope:
- Specs read:
- Date:

## Reference Sources

- Primary counterpart:
- Primary URL:
- Fallback counterparts inspected:
- Reason for fallback or N/A:

## Playwright Exploration Commands

```bash
playwright-cli -s=reference open <counterpart-url>
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-<component>-initial.yml
```

## Reference Evidence

| State or outcome | Command or action | Artifact | Notes |
| ---------------- | ----------------- | -------- | ----- |
| Basic            |                   |          |       |
| Focused          |                   |          |       |
| Disabled         |                   |          |       |
| Invalid or error |                   |          |       |
| Submit or reset  |                   |          |       |

## Observed Reference Outcomes

| Axis                  | Reference behavior | User-visible outcome | Accessibility outcome | Notes |
| --------------------- | ------------------ | -------------------- | --------------------- | ----- |
| Basic rendering       |                    |                      |                       |       |
| Label and description |                    |                      |                       |       |
| Invalid or error      |                    |                      |                       |       |
| I18n messages         |                    |                      |                       |       |
| Keyboard and focus    |                    |                      |                       |       |
| Live regions          |                    |                      |                       |       |
| Submit                |                    |                      |                       |       |
| Reset                 |                    |                      |                       |       |

## I18n Mapping

| User-facing text or locale-sensitive behavior | Source | Locale/direction cases | Tests or evidence | Status | Notes |
| --------------------------------------------- | ------ | ---------------------- | ----------------- | ------ | ----- |
| Label text                                    |        |                        |                   |        |       |
| Validation message                            |        |                        |                   |        |       |
| Status announcement                           |        |                        |                   |        |       |

## Accessibility Mapping

| Accessibility axis             | Required DOM/behavior | Adapter/core source | Tests or evidence | Status | Notes |
| ------------------------------ | --------------------- | ------------------- | ----------------- | ------ | ----- |
| Accessible name                |                       |                     |                   |        |       |
| Description/error relationship |                       |                     |                   |        |       |
| Keyboard/focus path            |                       |                     |                   |        |       |
| Live region                    |                       |                     |                   |        |       |
| Axe states                     |                       |                     |                   |        |       |

## Ars Contract Mapping

| Axis             | Status | API/contract stance | Agnostic or shared support | Adapter support needed | Widget and E2E support | Tests or evidence | Notes |
| ---------------- | ------ | ------------------- | -------------------------- | ---------------------- | ---------------------- | ----------------- | ----- |
| Basic rendering  |        |                     |                            |                        |                        |                   |       |
| Invalid or error |        |                     |                            |                        |                        |                   |       |
| Submit           |        |                     |                            |                        |                        |                   |       |
| Reset            |        |                     |                            |                        |                        |                   |       |

Allowed statuses: `Supported`, `RendererSpecific`, `ContractGap`,
`NotApplicable`, `IntentionallyDifferent`.

Allowed API/contract stances: `IdiomaticEquivalent`, `SameNativeBehavior`,
`HigherLevelComposition`, `OutOfScopeApiShape`.

## Parity Audit Loop

Run after the first implementation works and before handoff. Complete at least
three passes and keep looping while any row is `Unknown`, `Unverified`,
`ContractGap`, `AdapterApiGap`, or `WidgetOnlyWorkaround`.

### Pass 1: Reference Outcome Pass

- Date:
- Findings:
- Rows added or split:
- Remaining gaps:

### Pass 2: Consumer Reality Pass

- Date:
- Actual adapter usage checked:
- Widgets crates checked:
- Raw-control or duplicated-policy workarounds:
- Example-owned logic audit:
  - Consumer-owned only:
  - Component logic found:
  - API gaps opened from examples:
- Hardcoded user-facing text found:
- Remaining gaps:

### Pass 3: I18n, A11y, And Test Proof Pass

- Date:
- Locale/direction proof:
- Accessibility proof:
- Adapter wasm proof:
- E2E/browser outcome proof:
- Remaining gaps:

### Additional Passes

| Pass | Date | Focus | Findings | Remaining gaps |
| ---- | ---- | ----- | -------- | -------------- |
|      |      |       |          |                |

## Final Outcome Matrix

| Reference outcome | Final status | API/contract stance | Reference proof | Local proof | Adapter tests | E2E/browser proof | I18n proof | A11y proof | Notes |
| ----------------- | ------------ | ------------------- | --------------- | ----------- | ------------- | ----------------- | ---------- | ---------- | ----- |
|                   |              |                     |                 |             |               |                   |            |            |       |

Allowed final statuses: `ReferenceOutcomeMatched`,
`IntentionallyDifferent`, `OutOfScopeWithReason`.

Allowed API/contract stances: `IdiomaticEquivalent`, `SameNativeBehavior`,
`HigherLevelComposition`, `OutOfScopeApiShape`.

## Contract Gaps Before Coding

| Gap | Evidence | Required fix | Spec update needed |
| --- | -------- | ------------ | ------------------ |
|     |          |              |                    |

## Implementation Sketch

1. Agnostic/spec changes:
2. Leptos adapter changes:
3. Dioxus adapter changes:
4. Widget changes:
5. E2E/browser harness changes:

## Verification Plan

- Focused tests:
- I18n tests:
- Accessibility tests:
- E2E command:
- Widget smoke:
- Browser reference/local comparison:
- `cargo xtask lint adapter-parity`
- `cargo xclippy`

## Handoff Update

- Local evidence paths:
- Parity audit loop passes completed:
- Final outcome counts:
- Rows still Unknown/Unverified/ContractGap/AdapterApiGap/WidgetOnlyWorkaround:
- Final parity status:
- Final i18n status:
- Final accessibility status:
- Remaining `NotApplicable` axes:
- Remaining `IntentionallyDifferent` axes:
- Remaining risks:
