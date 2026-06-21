# Tabs Adapter Reference Exploration Sketch

## Task

- Issues: local Tabs adapter audit; historical implementation issues are #364 (Leptos) and #453 (Dioxus)
- Component: Tabs
- Category: navigation
- Adapters in scope: Leptos, Dioxus
- Specs read:
  - `spec/components/navigation/tabs.md`
  - `spec/leptos-components/navigation/tabs.md`
  - `spec/dioxus-components/navigation/tabs.md`
  - `docs/implementation/adapter-component-delivery.md`
  - `docs/implementation/adapter-components/README.md`
  - `docs/implementation/adapter-components/01-before-you-code.md`
  - `docs/implementation/adapter-components/02-adapter-api-and-wiring.md`
  - `docs/implementation/adapter-components/03-framework-rules.md`
  - `docs/implementation/adapter-components/04-adapter-tests.md`
  - `docs/implementation/adapter-components/05-e2e-fixtures-and-harnesses.md`
  - `docs/implementation/adapter-components/06-widgets-examples.md`
  - `docs/implementation/adapter-components/07-parity-review.md`
  - `docs/implementation/adapter-components/08-validation-and-pr-closeout.md`
  - `docs/implementation/adapter-components/09-browser-parity-harness.md`
  - `docs/implementation/adapter-components/10-reference-exploration-sketch.md`
  - `docs/implementation/adapter-components/11-i18n-and-a11y-support.md`
  - `docs/implementation/adapter-components/12-parity-audit-loop.md`
  - `docs/implementation/adapter-components/13-composition-integration.md`
  - `docs/implementation/adapter-components/14-retrofit-audits.md`
  - `docs/implementation/adapter-components/checklists/component-delivery.md`
  - `docs/implementation/adapter-components/checklists/e2e-feature-matrix.md`
  - `docs/implementation/adapter-components/checklists/widgets-visual-review.md`
  - `examples/widgets-ownership.md`
- Date: 2026-06-16

## Reference Sources

- Primary counterpart: React Aria Tabs
- Primary URL: `https://react-aria.adobe.com/Tabs`
- Fallback counterparts inspected:
  - Radix UI Tabs: `https://www.radix-ui.com/primitives/docs/components/tabs`
  - Ark UI Tabs: `https://ark-ui.com/docs/components/tabs`
- Reason for fallback or N/A: React Aria covers the required public outcomes for Tabs, including orientation, automatic/manual keyboard activation, controlled/default selection, disabled keys, dynamic collections, link tabs, panel mount policy, and styling data attributes. Radix and Ark remain fallback sources for terminology and activation-loop parity only.

## Playwright Exploration Commands

```bash
playwright-cli -s=reference open https://react-aria.adobe.com/Tabs
playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-tabs-initial.yml
playwright-cli -s=reference eval "() => ({tabs: Array.from(document.querySelectorAll('[role=tab]')).slice(0, 16).map(tab => ({text: tab.textContent?.trim(), selected: tab.getAttribute('aria-selected'), disabled: tab.getAttribute('aria-disabled'), tabindex: tab.getAttribute('tabindex'), dataset: {...tab.dataset}})), tablists: Array.from(document.querySelectorAll('[role=tablist]')).slice(0, 6).map(list => ({label: list.getAttribute('aria-label'), orientation: list.getAttribute('aria-orientation'), dataset: {...list.dataset}})), panels: Array.from(document.querySelectorAll('[role=tabpanel]')).slice(0, 8).map(panel => ({text: panel.textContent?.trim()?.slice(0, 80), hidden: panel.hidden, inert: panel.inert, dataset: {...panel.dataset}}))})"
```

## Reference Evidence

| State or outcome       | Command or action                            | Artifact                                     | Notes                                                                                                                                   |
| ---------------------- | -------------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Basic settings tabs    | Open React Aria page and snapshot            | `.playwright-cli/reference-tabs-initial.yml` | First demo renders labelled horizontal tabs with a selected `General` tab and form content in the selected panel.                       |
| Roving selection state | DOM eval over first 16 `[role=tab]` elements | eval output in implementation log            | Selected tabs have `aria-selected="true"` and `tabindex="0"`; unselected enabled tabs have `aria-selected="false"` and `tabindex="-1"`. |
| Disabled item          | DOM eval over selection demo                 | eval output in implementation log            | Disabled `Search` tab has `aria-disabled="true"`, no `tabindex`, and `data-disabled`.                                                   |
| Dynamic collections    | React Aria Content example                   | `.playwright-cli/reference-tabs-initial.yml` | Dynamic tabs are rendered from an item collection and panels are mapped from the same collection.                                       |
| Link tabs              | React Aria Links section                     | `.playwright-cli/reference-tabs-initial.yml` | Link behavior is a tab trigger rendering concern, not a separate state-machine outcome.                                                 |
| Styling states         | React Aria API docs                          | `.playwright-cli/reference-tabs-initial.yml` | React Aria documents data attributes for selected, hovered, pressed, focused, focus-visible, disabled, and inert panel states.          |

## Observed Reference Outcomes

| Axis                         | Reference behavior                                                             | User-visible outcome                                                | Accessibility outcome                                                                      | Notes                                                                                                              |
| ---------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Basic rendering              | `<Tabs>` owns tablist, tabs, panels, and selected panel                        | One selected tab and one visible panel                              | `role=tablist`, `role=tab`, `role=tabpanel`, `aria-selected`, roving `tabindex`            | ars-ui adapter primitives express this through `Root` plus collection-driven `List` / `Panels` over `Tab` rows.    |
| Orientation                  | `orientation` supports horizontal and vertical                                 | Arrow axis and orientation styling change                           | `aria-orientation` and orientation data attr                                               | ars-ui supports `Orientation` prop and tests vertical output/navigation.                                           |
| Activation mode              | `keyboardActivation` supports automatic/manual                                 | Focus may select immediately or wait for Enter/Space                | selected tab can differ from focused tab in manual mode                                    | ars-ui supports `ActivationMode`.                                                                                  |
| Disabled tabs                | `disabledKeys` / disabled item                                                 | Disabled tab remains visible but cannot be selected/focused         | `aria-disabled`, no roving focus entry                                                     | ars-ui supports prop and row-level disabled state.                                                                 |
| Controlled/default selection | `selectedKey` and `defaultSelectedKey`                                         | Selection can be app-owned or adapter-owned                         | selected state remains synchronized with external value                                    | ars-ui supports controlled `value` and `default_value`.                                                            |
| Dynamic rows                 | `items` collection renders tabs and panels                                     | Rows can be added/removed while preserving semantics                | tablist relationships follow live collection                                               | ars-ui supports `TabsSource` with owned rows and reactive stores.                                                  |
| Link tabs                    | `href` tab renders link trigger                                                | Tab can behave as navigation link                                   | tab remains part of tablist semantics                                                      | ars-ui supports `Tab::link`.                                                                                       |
| Panel mount policy           | `shouldForceMount` keeps inactive panels inert                                 | Inactive panels may stay mounted for animation or be unmounted/lazy | inactive panel not interactive                                                             | ars-ui supports `lazy_mount` and `unmount_on_exit`; forced inert panels are represented by hidden inactive panels. |
| Visual state hooks           | data attributes for selected, focus-visible, disabled, hovered, pressed, inert | Styling can target state and anatomy                                | state attrs reflect interaction state                                                      | ars-ui emits `data-ars-*` anatomy/state attrs. Hover/press are not currently a Tabs-specific public outcome.       |
| Closable tabs                | Not a React Aria Tabs baseline feature                                         | Browser/editor-style tab close affordance                           | close trigger has localized accessible label and optional consumer-supplied visual content | ars-ui intentionally extends baseline with closable tabs while keeping close semantics adapter-owned.              |
| Reorderable tabs             | Not a React Aria Tabs baseline feature                                         | Drag and Ctrl+Arrow reorder tab order                               | draggable roledescription and live reorder announcement                                    | ars-ui intentionally extends baseline with reorderable tabs.                                                       |

## I18n Mapping

| User-facing text or locale-sensitive behavior | Source                                              | Locale/direction cases                       | Tests or evidence                                                                               | Status    | Notes                                                                                                   |
| --------------------------------------------- | --------------------------------------------------- | -------------------------------------------- | ----------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------- |
| Tab labels                                    | Consumer `TabLabel`: static, translated, or dynamic | Translated keys use adapter provider locale  | `close_label_uses_live_provider_messages_after_locale_changes`; widgets use `Translate` and `t` | Supported | Labels are semantic sources used for visible trigger fallback, close labels, and reorder announcements. |
| Close trigger accessible label                | `tabs::Messages::close_tab_label`                   | Provider messages update after locale switch | Leptos/Dioxus wasm close-label locale tests                                                     | Supported | ars-ui owns this text because it creates the close trigger.                                             |
| Reorder live announcement                     | `tabs::Messages::reorder_announce_label`            | Provider locale flows through message bundle | wasm live-region reorder tests                                                                  | Supported | Reorder is an ars-ui extension over React Aria baseline.                                                |
| Widget explanatory copy                       | Widget-local `Translate` enums                      | en-US and pt-BR messages present             | all six navigation widgets define translated text                                               | Supported | Consumer-owned copy, not component-owned policy.                                                        |
| Direction-sensitive keyboard behavior         | `Direction` prop / provider auto direction          | LTR, RTL, and auto direction cases           | core tests plus adapter wasm direction tests                                                    | Supported | Horizontal ArrowLeft/ArrowRight behavior follows direction.                                             |

## Accessibility Mapping

| Accessibility axis  | Required DOM/behavior                                                              | Adapter/core source                                                     | Tests or evidence                                                                              | Status    | Notes                                                                                                                               |
| ------------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Accessible names    | Tablist and tabs need usable labels/text                                           | `Tab` label content and semantic `label_text`                           | adapter SSR tests and E2E accessibility-tree assertions                                        | Supported | No component-owned hardcoded English.                                                                                               |
| Relationships       | Tabs control panels, panels are labelled by tabs, and tablist owns registered tabs | `Api::list_attrs`, `Api::tab_attrs`, `Api::panel_attrs`, `ComponentIds` | SSR tests for `aria-controls` / `aria-labelledby`; core and wasm tests for `aria-owns` updates | Supported | Relationships are core-derived.                                                                                                     |
| Keyboard/focus path | Arrow/Home/End, manual Enter/Space, focus-visible modality                         | core keyboard API plus adapter DOM focus dispatch                       | wasm keyboard tests and E2E `run_tabs_flow`                                                    | Supported | Focus dispatch is renderer-specific.                                                                                                |
| Disabled behavior   | Disabled tabs are visible but skipped and inactive                                 | core disabled key set from props + row metadata                         | SSR, wasm, and E2E disabled assertions                                                         | Supported | Disabled rows do not render active close trigger.                                                                                   |
| Closable behavior   | Delete/Backspace and close trigger request close only for closable enabled tabs    | core `CloseTab`, adapter callback dispatch                              | SSR/wasm close tests and E2E delete flow                                                       | Supported | Consumer applies the actual row removal for controlled stores; consumer-supplied close content is visual-only.                      |
| Reorder behavior    | Drag and Ctrl+Arrow reorder enabled tabs and announce new position                 | core reorder helpers plus adapter DOM drag/keyboard glue                | wasm reorder tests, drag-image tests, and E2E drag/Ctrl+Arrow flow                             | Supported | Adapter owns DOM drag event conversion and custom shell-clone drag images.                                                          |
| Axe states          | Page must remain axe-clean after visible Tabs state is reached                     | E2E harness                                                             | `run_axe(driver)` after navigation category is selected                                        | Supported | Existing harness runs axe once after reaching category; additional reached-state axe can be added if future UX findings require it. |

## Ars Contract Mapping

| Axis                         | Status    | API/contract stance | Agnostic or shared support                        | Adapter support needed                                                                                                                               | Widget and E2E support                                                        | Tests or evidence                                | Notes                                                                                                                                                                                                                      |
| ---------------------------- | --------- | ------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Basic rendering              | Supported | IdiomaticEquivalent | `tabs::Machine`, `Api`, `Part`, `TabRegistration` | Leptos/Dioxus `Root`, `List`, `Panels`, `TabShell`, `Panel`, and `LiveRegion` compose the public anatomy; `List` owns the internal indicator node    | all six widgets and navigation E2E fixture                                    | SSR, wasm, E2E                                   | Adapter primitives are collection-driven so consumers do not repeat tab keys.                                                                                                                                              |
| Orientation                  | Supported | IdiomaticEquivalent | `Orientation` prop and attrs                      | adapter prop and keyboard mapping                                                                                                                    | widget copy and E2E keyboard                                                  | SSR/wasm                                         | No gap.                                                                                                                                                                                                                    |
| Activation mode              | Supported | IdiomaticEquivalent | `ActivationMode`                                  | adapter event conversion                                                                                                                             | widget copy and wasm tests                                                    | SSR/wasm                                         | No gap.                                                                                                                                                                                                                    |
| Disabled tabs                | Supported | IdiomaticEquivalent | disabled key set and row metadata helpers         | adapter row metadata sync                                                                                                                            | widgets + E2E                                                                 | SSR/wasm/E2E                                     | No gap.                                                                                                                                                                                                                    |
| Controlled/default selection | Supported | IdiomaticEquivalent | `Bindable<Option<Key>>`                           | Leptos signals, Dioxus prop sync                                                                                                                     | wasm tests                                                                    | wasm                                             | No gap.                                                                                                                                                                                                                    |
| Dynamic rows                 | Supported | IdiomaticEquivalent | `SetTabs`, metadata helpers                       | Leptos `Field`, Dioxus `ReadStore`                                                                                                                   | E2E fixture and wasm tests                                                    | wasm/E2E                                         | No gap.                                                                                                                                                                                                                    |
| Link tabs                    | Supported | SameNativeBehavior  | core still owns tab semantics                     | adapter renders `a` when `Tab::link` is set                                                                                                          | SSR tests                                                                     | SSR                                              | No gap.                                                                                                                                                                                                                    |
| Panel mount policy           | Supported | IdiomaticEquivalent | `lazy_mount`, `unmount_on_exit` props and attrs   | adapter render branch                                                                                                                                | SSR tests                                                                     | SSR                                              | No gap.                                                                                                                                                                                                                    |
| Styling customization        | Supported | IdiomaticEquivalent | `Part` and `data-ars-*` attrs                     | public adapter primitives, typed root renderers, root class/global attrs, part attrs, and selector-based styling for private behavior-critical nodes | widgets CSS/Tailwind target public data attrs and widget browser smoke checks | SSR/E2E/widget visual assertions                 | Monolithic adapter `Tabs` is superseded; closed visual Tabs now live in styled component crates; Tailwind users target `tab-close-trigger` and `tab-indicator` from public ancestors or edit the Tailwind source template. |
| Closable tabs                | Supported | IdiomaticEquivalent | close event, successor helpers, messages          | close trigger rendering/callbacks plus optional close glyph/content                                                                                  | widgets + E2E                                                                 | SSR/wasm/E2E                                     | ars-ui extension over React Aria baseline; default glyph is a fallback, not the only visual API.                                                                                                                           |
| Reorderable tabs             | Supported | IdiomaticEquivalent | reorder event/plan/helpers/messages               | drag/Ctrl+Arrow DOM glue                                                                                                                             | widgets + E2E                                                                 | wasm/E2E                                         | ars-ui extension over React Aria baseline.                                                                                                                                                                                 |
| Prelude consumer ergonomics  | Supported | IdiomaticEquivalent | public adapter row/source types                   | prelude exports the `tabs` module only; consumers use namespaced `tabs::...` items                                                                   | widgets/fixtures should import the module through prelude                     | `tabs_rows_are_reexported_for_prelude_consumers` | Gap found and remediated by this audit without flattening part names.                                                                                                                                                      |

## Parity Audit Loop

### Pass 1: Reference Outcome Pass

- Date: 2026-06-16
- Findings:
  - React Aria baseline outcomes are represented by the ars-ui Tabs primitive API: basic rendering, orientation, activation mode, disabled keys, controlled/default selection, dynamic collections, link tabs, panel mount policy, and state styling attrs.
  - ars-ui intentionally supports additional closable and reorderable outcomes not present in the React Aria baseline Tabs docs.
  - Superseded: the monolithic adapter API was acceptable for the first audit pass, but the latest adapter workflow requires public unstyled primitives plus styled closed components in `ars-*-components`.
- Rows added or split:
  - Split baseline disabled, dynamic rows, link tabs, panel mount policy, closable extension, and reorderable extension into independent rows.
- Remaining gaps:
  - Prelude consumer ergonomics gap: the adapter preludes did not expose the `tabs` module, forcing deep imports in widgets/fixtures instead of the current namespaced `tabs::...` convention.
  - Adapter spec drift: the Leptos spec still described older non-signal optional props and omitted `class`; the Dioxus spec omitted root `GlobalAttributes`, `TabLabel`, and the module-scoped row/source exports.

### Pass 2: Consumer Reality Pass

- Date: 2026-06-16
- Actual adapter usage checked:
  - `crates/ars-leptos/src/navigation/tabs.rs`
  - `crates/ars-dioxus/src/navigation/tabs.rs`
  - `crates/ars-e2e/fixtures/leptos/src/categories/navigation.rs`
  - `crates/ars-e2e/fixtures/dioxus/src/categories/navigation.rs`
  - `crates/ars-e2e/fixtures/leptos/src/main.rs`
  - `crates/ars-e2e/fixtures/dioxus/src/main.rs`
- Widgets crates checked:
  - `examples/widgets-leptos`
  - `examples/widgets-dioxus`
  - `examples/widgets-leptos-css`
  - `examples/widgets-dioxus-css`
  - `examples/widgets-leptos-tailwind`
  - `examples/widgets-dioxus-tailwind`
- Raw-control or duplicated-policy workarounds:
  - None found for Tabs behavior. Widgets supply sample row data, translated copy, and styling only.
- Example-owned logic audit:
  - Consumer-owned only: sample tabs, translated panel copy, root panel layout, CSS/Tailwind visual styling through public parts and stable `data-ars-part` selectors.
  - Component logic found: none for selection, roving focus, close policy, reorder policy, ARIA relationships, or messages.
  - API gaps opened from examples: expose the `tabs` module from the prelude while keeping all parts and row/source helpers namespaced.
- Hardcoded user-facing text found:
  - No component-owned hardcoded English. Widget prose is translated through local `Translate` enums.
- Remaining gaps:
  - Deep imports for `Tab`/`Tabs` in widgets/fixtures before remediation.

### Pass 3: I18n, A11y, And Test Proof Pass

- Date: 2026-06-16
- Locale/direction proof:
  - Provider-backed close-label locale updates are covered by wasm tests.
  - Direction and orientation keyboard behavior are covered by core and adapter wasm tests.
- Accessibility proof:
  - SSR tests cover roles, selected state, disabled state, close labels, relationships, roledescription, and live region.
  - E2E covers accessibility-tree role/name assertions and axe after entering the Navigation category.
- Adapter wasm proof:
  - Existing wasm tests cover keyboard, pointer, focus-visible modality, click selection, manual activation, close hotkeys/clicks, disabled guards, controlled value, store push/pop/update, locale messages, auto direction, signal-backed orientation, and signal-backed reorderable behavior.
- E2E/browser outcome proof:
  - `crates/ars-e2e/src/navigation/tabs.rs::run_tabs_flow` covers pointer selection, keyboard selection, focus-visible indicators, panel tab order, touch/pen pointer paths, drag reorder, Ctrl+Arrow reorder, close via Delete, visible order, and active focus after mutation.
- Remaining gaps:
  - Add green proof for new prelude tests and run focused adapter/widget/E2E checks.

### Additional Passes

| Pass | Date       | Focus                       | Findings                                                                                                                                                                                                                                                                                          | Remaining gaps                                   |
| ---- | ---------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| 4    | 2026-06-16 | Adapter boundary quick scan | Dioxus fallback-hook search found no hooks hidden in `unwrap_or_else` / `map_or_else` in `tabs.rs`; duplicated helper families are currently renderer-bound by store, event, attr, node, and DOM APIs or already use shared core helpers (`TabMeta`, registrations, disabled keys, reorder plan). | Validate with `cargo xtask lint adapter-parity`. |

## Final Outcome Matrix

| Reference outcome           | Final status            | API/contract stance | Reference proof                                                      | Local proof                                                                                | Adapter tests                                                                                        | E2E/browser proof                                                        | I18n proof                          | A11y proof                                             | Notes                                                                                 |
| --------------------------- | ----------------------- | ------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| Basic labelled tabs         | ReferenceOutcomeMatched | IdiomaticEquivalent | `.playwright-cli/reference-tabs-initial.yml`                         | `cargo xtask e2e widgets --example leptos`, `--example dioxus`, plus CSS/Tailwind variants | SSR `translated_tab_keys_render_default_labels` / `static_tab_labels_render_without_translate_bound` | `run_tabs_flow` and widget smoke Tabs assertions                         | translated tab keys and widget text | SSR relationships + E2E axe                            | Primitive API preserves outcome; styled components provide the closed visual wrapper. |
| Orientation                 | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria `orientation` docs and DOM eval                           | adapter attrs and widgets                                                                  | `vertical_orientation_propagates_to_aria_and_data_attrs`                                             | keyboard axis wasm tests                                                 | N/A                                 | `aria-orientation` tests                               | No gap.                                                                               |
| Automatic/manual activation | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria `keyboardActivation` docs                                 | adapter prop                                                                               | manual activation SSR/wasm tests                                                                     | E2E keyboard path                                                        | N/A                                 | selected/focused assertions                            | No gap.                                                                               |
| Disabled keys               | ReferenceOutcomeMatched | IdiomaticEquivalent | DOM eval showing disabled `Search` tab                               | disabled widget tab                                                                        | disabled SSR/wasm tests                                                                              | E2E disabled tab remains in order                                        | translated disabled panel copy      | `aria-disabled` tests                                  | No gap.                                                                               |
| Dynamic collections         | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria dynamic collection example                                | reactive store tests                                                                       | store push/pop/update wasm tests                                                                     | E2E close/reorder mutation flow                                          | N/A                                 | `aria-owns` update tests                               | No gap.                                                                               |
| Link tabs                   | ReferenceOutcomeMatched | SameNativeBehavior  | React Aria Links section                                             | adapter `Tab::link`                                                                        | link SSR tests                                                                                       | N/A; static/link semantics are SSR-covered                               | consumer-owned link text            | sibling close-trigger link tests                       | No gap.                                                                               |
| Panel mount policy          | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria `shouldForceMount` docs                                   | adapter `lazy_mount` / `unmount_on_exit`                                                   | lazy mount SSR tests                                                                                 | N/A; render policy is SSR-covered                                        | N/A                                 | hidden panel attrs                                     | No gap.                                                                               |
| Styling states              | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria data-state docs                                           | CSS/Tailwind widgets and widget visual smoke                                               | SSR data-attr tests                                                                                  | focus/selected indicator E2E assertions and widget computed-style deltas | N/A                                 | data attrs + axe                                       | No compound primitive or styled-template expansion required now.                      |
| Closable tabs               | IntentionallyDifferent  | IdiomaticEquivalent | Not a React Aria baseline Tabs outcome                               | ars-ui widgets and fixtures                                                                | close SSR/wasm tests                                                                                 | E2E Delete close flow                                                    | close-label message bundle          | localized close label and keyboard guards              | Deliberate ars-ui extension.                                                          |
| Reorderable tabs            | IntentionallyDifferent  | IdiomaticEquivalent | Not a React Aria baseline Tabs outcome                               | ars-ui widgets and fixtures                                                                | reorder SSR/wasm tests plus Leptos/Dioxus drag-image regressions                                     | E2E drag and Ctrl+Arrow reorder                                          | reorder live message bundle         | roledescription + live region + shell-clone drag image | Deliberate ars-ui extension.                                                          |
| Prelude consumer ergonomics | ReferenceOutcomeMatched | IdiomaticEquivalent | React Aria exports compound row types directly from component module | adapter preludes export the `tabs` module, not flattened part names                        | prelude tests prove namespaced `tabs::...` access                                                    | widgets/fixtures import the module through prelude                       | N/A                                 | N/A                                                    | Gap remediated by audit while preserving the no-flattened-parts rule.                 |

## Contract Gaps Before Coding

| Gap                                                                                                  | Evidence                                                                                                                                                                  | Required fix                                                                                                                                                                                                                                                                                                                                                                                                     | Spec update needed                                                                                                           |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Tabs row/source and primitive part types were not available through the adapter module/prelude shape | Widgets and fixtures imported deep adapter modules despite adapter workflow preferring `prelude::*` plus namespaced `tabs::...` for user-facing primitive APIs            | Re-export only the `tabs` module through adapter preludes and keep public parts plus row/source helpers namespaced (`tabs::Root`, `tabs::List`, `tabs::Panels`, `tabs::TabShell`, `tabs::Trigger`, `tabs::CloseTrigger`, `tabs::Panel`, `tabs::LiveRegion`, `tabs::TabRenderItem`, `tabs::Tab`, `tabs::TabLabel`, `tabs::TabsSource`, and Dioxus `tabs::RootProps`); keep the raw-attrs indicator helper private | No. Adapter prelude convention already covers user-facing configuration and component modules; this is implementation drift. |
| Adapter specs described stale public API details                                                     | `spec/leptos-components/navigation/tabs.md` used older optional prop shapes and no `class`; `spec/dioxus-components/navigation/tabs.md` omitted root attrs and `TabLabel` | Sync specs to current Leptos/Dioxus public adapter APIs and prelude surface                                                                                                                                                                                                                                                                                                                                      | Yes. Landed in adapter specs.                                                                                                |

## Implementation Sketch

Superseded outcome: the earlier audit conclusion that the adapter should
keep a monolithic public `Tabs` API is no longer current. The latest adapter
workflow requires unstyled public primitives in `ars-leptos` / `ars-dioxus`
and closed ready-made visual components only in `ars-leptos-components` /
`ars-dioxus-components`.

1. Agnostic/spec changes: keep the core ownership improvements from this
   audit (`aria-owns`, reorderable prop sync, shell-clone drag image
   contract) and sync adapter specs to the primitive surface.
2. Leptos adapter changes: expose `Root`, `List`, `Panels`,
   `TabRenderItem`, `TabShell`, `Trigger`, `CloseTrigger`, `Panel`, and
   `LiveRegion` from the adapter module; keep the raw-attrs indicator helper
   private. `TabShell<K>` publishes the typed row through framework context so
   descendant `Trigger<K>` and `CloseTrigger<K>` receive the item without
   cloned `item` props while still preserving adapter-owned trigger, close,
   focus, reorder, and ARIA policy.
3. Dioxus adapter changes: expose the same public primitive set plus
   `RootProps`; use the same `TabShell<K>` typed row context boundary; keep
   Dioxus hook order stable and browser-only `--features web` behavior
   covered by wasm tests.
4. Styled component changes: move the ergonomic closed `Tabs` into
   `ars-leptos-components::navigation::tabs::{css,tailwind}` and
   `ars-dioxus-components::navigation::tabs::{css,tailwind}`, where it
   composes the adapter primitives.
5. Widget/E2E changes: plain, CSS, and Tailwind widgets include full
   primitive composition examples with every public part; styled high-level
   Tabs remains available in `ars-*-components` as the safer closed source
   template. E2E fixtures compose adapter primitives directly.

## Verification Plan

- Focused tests:
  - `cargo test -p ars-leptos --lib tabs_rows_are_reexported_for_prelude_consumers`
  - `cargo test -p ars-dioxus --lib tabs_rows_are_reexported_for_prelude_consumers`
  - `cargo test -p ars-leptos --features ssr --test tabs`
  - `cargo test -p ars-dioxus --test tabs`
  - `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test -p ars-leptos --features csr --target wasm32-unknown-unknown --test tabs_wasm`
  - `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner WASM_BINDGEN_TEST_ONLY_WEB=1 cargo test -p ars-dioxus --features web --target wasm32-unknown-unknown --test tabs_wasm`
- I18n tests:
  - covered by existing Tabs wasm locale-message tests in focused wasm run
- Accessibility tests:
  - covered by existing SSR and E2E runs
- E2E command:
  - `cargo xtask e2e navigation --adapter leptos`
  - `cargo xtask e2e navigation --adapter dioxus`
- Widget smoke:
  - `cargo check --manifest-path examples/widgets-leptos/Cargo.toml`
  - `cargo check --manifest-path examples/widgets-dioxus/Cargo.toml`
  - `cargo check --manifest-path examples/widgets-leptos-css/Cargo.toml`
  - `cargo check --manifest-path examples/widgets-dioxus-css/Cargo.toml`
  - `cargo check --manifest-path examples/widgets-leptos-tailwind/Cargo.toml`
  - `cargo check --manifest-path examples/widgets-dioxus-tailwind/Cargo.toml`
- Public widget browser smoke:
  - `cargo xtask e2e widgets --example leptos`
  - `cargo xtask e2e widgets --example dioxus`
  - `cargo xtask e2e widgets --example leptos-css`
  - `cargo xtask e2e widgets --example dioxus-css`
  - `cargo xtask e2e widgets --example leptos-tailwind`
  - `cargo xtask e2e widgets --example dioxus-tailwind`
- Browser reference/local comparison:
  - reference artifact: `.playwright-cli/reference-tabs-initial.yml`
  - local proof: E2E navigation harness and widget checks
- `cargo xtask lint adapter-parity`
- `cargo xfmt`
- `cargo xclippy`

## Handoff Update

- Local evidence paths:
  - `.playwright-cli/reference-tabs-initial.yml`
  - Tabs-focused Rust, wasm, E2E, widget browser smoke, parity, spec, formatting, clippy, and coverage command output from this remediation session
- Parity audit loop passes completed:
  - Pass 1 reference outcomes
  - Pass 2 consumer reality
  - Pass 3 i18n/a11y/test proof
  - Pass 4 adapter-boundary quick scan
- Final outcome counts:
  - `ReferenceOutcomeMatched`: 9
  - `IntentionallyDifferent`: 2
  - `OutOfScopeWithReason`: 0
- Rows still Unknown/Unverified/ContractGap/AdapterApiGap/WidgetOnlyWorkaround:
  - 0
- Final parity status:
  - outcome-complete with validation gates passing
- Final i18n status:
  - Supported through consumer text, `Translate`, and component message bundles
- Final accessibility status:
  - Supported through core attrs, adapter DOM tests, wasm tests, and E2E axe/role checks
  - Remediated axe-discovered nested interactive/ownership drift by adding `TabShell` anatomy and making close affordances non-roving pointer affordances adjacent to tab triggers
  - Remediated latest-main widget-smoke finding by rendering a built-in SVG glyph inside the close affordance, so plain unstyled widgets expose a visible, nonzero close target without CSS pseudo-content
  - Follow-up reference check against React Aria, Radix, Shadcn, Ark, and Chakra found no first-class close-trigger visual API; Tabs now treats the built-in close SVG as a fallback and lets consumers provide visual close content while preserving adapter-owned semantics
  - Remediated old-worktree drag-preview and ownership findings by moving tablist `aria-owns` into the agnostic API and adding Leptos/Dioxus shell-clone drag-image tests
- Remaining `NotApplicable` axes:
  - Form submit/reset: Tabs is not a form control; panels may contain forms as consumer content
  - Validation/error: Tabs owns no validation policy
  - Loading/empty: not a Tabs primitive responsibility in current spec
- Remaining `IntentionallyDifferent` axes:
  - Closable tabs
  - Reorderable tabs
- Remaining risks:
  - Full widget browser smoke beyond navigation E2E is not a separate command in the current tree; navigation E2E plus widget cargo checks remain the available proof paths.
