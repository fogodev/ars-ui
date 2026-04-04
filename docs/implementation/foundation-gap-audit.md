# Foundation Gap Audit and Backlog Reset

This audit resets the implementation backlog after the initial seed tasks landed only the thinnest crate shells. It explains why [issue #24](https://github.com/fogodev/ars-ui/issues/24) is premature, identifies the missing middle-layer contracts, and defines a corrected foundation-first task sequence before any adapter utility slice decomposition resumes.

## Summary

- `#24` is blocked because the repo does not yet implement the shared connect, anatomy, provider, forms, and DOM contracts that the first utility slice assumes.
- The earlier seed backlog correctly created crate shells, but it stopped before several spec-defined contracts became issue-backed work.
- The corrected sequence below promotes those missing contracts into explicit, issue-ready tasks sized at `5` points or less.
- The first post-audit implementation task should come from `ars-core`, not from adapter utility components.

## Gap Matrix

| Area                                       | Current implemented surface                                                                                                                                                                                                                                                                                         | Spec-required surface                                                                                                                                                                                                                                    | Blocker impact                                                                                                                                                                 | Classification                                    |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| `ars-core` connect primitives              | [`crates/ars-core/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-core/src/lib.rs) defines `AttrMap` as `BTreeMap<String, String>` and does not expose typed HTML attrs, event names, or CSS properties.                                                                                               | `HtmlAttr`, `HtmlEvent`, `CssProperty`, `EventOptions`, and typed state-to-DOM primitives from [`spec/foundation/01-architecture.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/01-architecture.md):3.1.1-3.1.3.                              | Blocks every spec-compliant `connect()` API because specs use typed attrs and CSS properties, not raw strings.                                                                 | Must land before any utility-slice implementation |
| `ars-core` attribute model                 | `AttrMap` lacks `AttrValue`, boolean values, style storage, space-separated token merging, user-attr filtering, and style strategy support.                                                                                                                                                                         | `AttrMap`, `AttrValue`, `UserAttrs`, `StyleStrategy`, and merge semantics from [`spec/foundation/01-architecture.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/01-architecture.md):3.2-3.2.2.                                                | Blocks all spec-defined `*_attrs()` methods, adapter attr conversion, and `as_child`/composition behavior.                                                                     | Must land before any utility-slice implementation |
| `ars-a11y` ARIA bridge                     | [`crates/ars-a11y/src/aria/attribute.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-a11y/src/aria/attribute.rs) has a deferred TODO for `attr_name()`, `to_html_attr()`, `apply_to()`, and `AriaAttr`/`HtmlAttr` conversion impls pending `ars-core` typed attr work.                                         | `AriaAttribute` bridging helpers and conversions from [`spec/foundation/03-accessibility.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/03-accessibility.md):2.2.                                                                             | Blocks spec-compliant ARIA application and prevents downstream connect code from using the typed accessibility layer rather than raw attr wiring.                              | Can land before specific utilities only           |
| `ars-a11y` role and state helpers          | Baseline `AriaRole`, `AriaAttribute`, and `ComponentIds` exist, but the helper layer for applying roles and common ARIA states to typed `AttrMap`s is missing.                                                                                                                                                      | `apply_role`, `apply_aria`, `set_expanded`, `set_selected`, `set_checked`, `set_disabled`, `set_busy`, and `set_invalid` from [`spec/foundation/03-accessibility.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/03-accessibility.md):2.3-2.5. | Blocks reuse of the spec-defined accessibility patterns in component connect functions and encourages ad hoc ARIA wiring in downstream components.                             | Can land before specific utilities only           |
| Anatomy and derive support                 | [`crates/ars-derive/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-derive/src/lib.rs) contains empty `HasId` and `ComponentPart` derive macros; [`crates/ars-core/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-core/src/lib.rs) lacks the scope/data-attr helpers the spec relies on. | `ComponentPart` scope/name/all/data-attr behavior and derive generation from [`spec/foundation/01-architecture.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/01-architecture.md):4.1.                                                        | Blocks all anatomy enums in utility/component specs because `data-ars-scope` and `data-ars-part` generation are not available.                                                 | Must land before any utility-slice implementation |
| Provider and platform contracts            | No `PlatformEffects`, `ColorMode`, `NullPlatformEffects`, or shared provider-facing environment types exist in the current crates.                                                                                                                                                                                  | `PlatformEffects` and `ArsProvider` shared environment contract from [`spec/foundation/01-architecture.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/01-architecture.md):2.2.7 and 6.4.                                                      | Blocks effect execution, environment propagation, and the `ArsProvider` slice item itself. Some stateless utilities could proceed without it, but the slice as planned cannot. | Can land before specific utilities only           |
| `ars-forms` context and field association  | [`crates/ars-forms/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-forms/src/lib.rs) currently implements only validation primitives and `FieldState`.                                                                                                                                                 | `FormContext`, `ValidationMode`, `FieldDescriptors`, `FieldCtx`, hidden-input contracts, and server-error sync scaffolding from [`spec/foundation/07-forms.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/07-forms.md):5-7 and 12.6.          | Blocks `Field`, `Fieldset`, `Form`, `ToggleButton`, and later form-participating components, but not stateless utilities like `VisuallyHidden`.                                | Can land before specific utilities only           |
| `ars-forms` submit lifecycle               | No form submit machine or focus-on-error helper exists.                                                                                                                                                                                                                                                             | `form_submit::Machine`, server-error sync, and invalid-field focus flow from [`spec/foundation/07-forms.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/07-forms.md):8-10.                                                                     | Blocks `Form` and any adapter contract that relies on stateful submit/validation behavior.                                                                                     | Can land before specific utilities only           |
| `ars-dom` focus and focus-scope primitives | [`crates/ars-dom/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-dom/src/lib.rs) only defines `ScrollLockToken` and `PlatformFeatures`.                                                                                                                                                                | DOM focus querying, focus management, and concrete `FocusScope` support from [`spec/foundation/11-dom-utilities.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/11-dom-utilities.md):3.1-3.3.                                                  | Blocks `FocusScope` directly and also blocks any focus-management effects routed through `PlatformEffects`.                                                                    | Can land before specific utilities only           |
| `ars-dom` scroll locking                   | No reference-counted scroll lock implementation exists.                                                                                                                                                                                                                                                             | `ScrollLockManager`, low-level `acquire`/`release`, and public aliases from [`spec/foundation/11-dom-utilities.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/11-dom-utilities.md):5.2-5.4.                                                   | Not needed for the first utility slice, but required before overlay work starts.                                                                                               | Can wait until later slices                       |
| Interaction composition depth              | [`crates/ars-interactions/src/lib.rs`](/Users/ericson/Workspace/Rust/ars-ui/crates/ars-interactions/src/lib.rs) only does shallow overwrite merging.                                                                                                                                                                | Token-aware composition behavior expected by [`spec/foundation/05-interactions.md`](/Users/ericson/Workspace/Rust/ars-ui/spec/foundation/05-interactions.md):8.1-8.2 and by `as_child`/button specs.                                                     | Blocks `AsChild`, `Button`, and other composed interactive utilities, but not all utility work.                                                                                | Can land before specific utilities only           |

## Why `#24` Is Blocked

`#24` assumed the first utility slice could be decomposed directly into adapter-facing delivery cards. The audit shows that the current repo still lacks the shared contracts those cards would depend on:

- Utility specs already call for typed `HtmlAttr`, `AttrMap::set_bool`, style handling, and part-derived `data_attrs()`.
- `Field`, `Fieldset`, `Form`, and `ToggleButton` all assume `FormContext`, `FieldCtx`, hidden-input helpers, and submit lifecycle machinery that do not exist yet.
- `FocusScope` and any effectful component assume DOM focus helpers and `PlatformEffects` integration that are still missing.

Because of that, decomposing the utility slice now would create issue cards that are blocked on unstated prerequisites. `#24` should stay deferred until the replacement foundation tasks below exist as issue-backed work.

## Replacement Task Sequence

The tasks below are issue-ready replacements for the missing middle layer. They are ordered by dependency, not by epic label.

### [#31](https://github.com/fogodev/ars-ui/issues/31): Implement typed connect primitives in `ars-core`

- Points: `5`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#13`
- Spec refs:
  - `spec/foundation/01-architecture.md#31-typed-property-enums`
  - `spec/foundation/01-architecture.md#311-htmlattr`
  - `spec/foundation/01-architecture.md#312-htmlevent`
  - `spec/foundation/01-architecture.md#313-cssproperty`
- Goal: add the typed HTML attribute, event, and CSS property model used by all spec-defined `connect()` APIs.
- Out of scope: `AttrMap` merge/storage behavior, derive macros, adapter conversion code.
- Tests to add first:
  - Unit tests for `Display`/name serialization of representative `HtmlAttr`, `HtmlEvent`, and `CssProperty` values.
  - Unit tests for helpers such as `HtmlAttr::static_name()` and `data()`.
- Acceptance criteria:
  - `ars-core` exposes typed attr/event/style enums matching the spec naming contract.
  - Downstream crates can reference typed keys without raw string literals.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record the exact missing spec or dependency issue.

### [#32](https://github.com/fogodev/ars-ui/issues/32): Implement spec-compliant `AttrMap`, `AttrValue`, `UserAttrs`, and `StyleStrategy`

- Points: `5`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`
- Spec refs:
  - `spec/foundation/01-architecture.md#32-attrmap`
  - `spec/foundation/01-architecture.md#321-stylestrategy`
  - `spec/foundation/01-architecture.md#322-companion-stylesheet-ars-basecss`
- Goal: replace the stringly `AttrMap` shell with the spec-defined typed attribute and style container.
- Out of scope: adapter-side conversion into framework props, user-facing components.
- Tests to add first:
  - Unit tests for `set`, `set_bool`, `set_style`, `contains`, `get`, and `merge`.
  - Unit tests for space-separated token list dedup (`class`, `aria-labelledby`, `aria-describedby`).
  - Unit tests proving blocked `UserAttrs` keys are rejected.
- Acceptance criteria:
  - `AttrMap` stores typed attrs and styles with spec-defined merge behavior.
  - `StyleStrategy` exists with documented default behavior.
  - SSR-safe consumers can inspect attrs/styles without string-key ad hoc logic.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and name the missing `#31` surface or spec ambiguity.

### [#33](https://github.com/fogodev/ars-ui/issues/33): Complete the `ars-a11y` `AriaAttribute` ↔ `ars-core` bridge

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`, `#32`, `#15`
- Spec refs:
  - `spec/foundation/03-accessibility.md#22-ariaattribute-enum`
- Goal: finish the deferred bridging layer between `AriaAttribute` and the typed `ars-core` attr model.
- Out of scope: role/state helper convenience functions, component-specific connect code, validator logic.
- Tests to add first:
  - Unit tests for `AriaAttribute::attr_name()`, `to_html_attr()`, and `apply_to()`.
  - Unit tests for `From<AriaAttr> for AriaAttribute`, `TryFrom<HtmlAttr> for AriaAttribute`, and `From<&AriaAttribute> for AriaAttr`.
  - Unit tests proving removal semantics map to `AttrValue::None` for nullable ARIA attrs such as `Expanded(None)` and `Hidden(None)`.
- Acceptance criteria:
  - `ars-a11y` no longer carries deferred TODOs for the typed ARIA bridge.
  - `AriaAttribute` can apply itself to the spec-compliant `AttrMap` without raw string keys.
  - Downstream code can round-trip between `AriaAttribute`, `AriaAttr`, and `HtmlAttr::Aria(...)`.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record whether the blocker is missing typed `AttrMap` behavior or an accessibility-spec mismatch.

### [#34](https://github.com/fogodev/ars-ui/issues/34): Add `ars-a11y` role and common state helper APIs

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#32`, `#33`, `#15`
- Spec refs:
  - `spec/foundation/03-accessibility.md#23-role-assignment-patterns`
  - `spec/foundation/03-accessibility.md#25-state-and-property-management`
- Goal: add the shared helper layer for applying roles and common ARIA states to typed `AttrMap`s.
- Out of scope: full validator work, adapter-side enforcement, component-specific accessibility composition.
- Tests to add first:
  - Unit tests for `apply_role()` and `apply_aria()` against typed `AttrMap`.
  - Unit tests for `set_expanded`, `set_selected`, `set_checked`, `set_disabled`, `set_busy`, and `set_invalid`.
  - Unit tests verifying `set_invalid` clears `aria-errormessage` when no error ID is supplied.
- Acceptance criteria:
  - `ars-a11y` exposes the helper APIs that the accessibility spec uses in connect examples.
  - Components can apply standard ARIA role/state patterns without handwritten attr-key branching.
  - Removal behavior for nullable state attrs is aligned with the spec and typed `AttrMap`.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record whether the blocker is in `ars-core` attr semantics or unresolved accessibility helper scope.

### [#35](https://github.com/fogodev/ars-ui/issues/35): Implement `ComponentPart` scope/data-attr helpers and derive macros

- Points: `3`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`, `#14`
- Spec refs:
  - `spec/foundation/01-architecture.md#4-anatomy-system`
  - `spec/foundation/01-architecture.md#41-anatomy-definition`
- Goal: implement the actual `HasId` and `ComponentPart` derive output required by spec anatomy enums.
- Out of scope: adapter rendering, component-specific anatomy definitions.
- Tests to add first:
  - Proc-macro tests for `#[derive(HasId)]`.
  - Proc-macro tests for `#[derive(ComponentPart)]` including scope lookup, kebab-case part names, `all()`, and `data_attrs()`.
- Acceptance criteria:
  - Generated code matches the spec contract for `scope`, `name`, `all`, and representative instances.
  - Components can derive anatomy helpers without handwritten boilerplate.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record the exact missing `#31` attr type or macro constraint.

### [#36](https://github.com/fogodev/ars-ui/issues/36): Introduce shared provider and platform-effect contracts

- Points: `3`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`
- Spec refs:
  - `spec/foundation/01-architecture.md#227-platformeffects-trait`
  - `spec/foundation/01-architecture.md#64-arsprovider`
- Goal: add the shared environment-side contract types that adapters and effect closures depend on before `ArsProvider` itself is implemented.
- Out of scope: Leptos/Dioxus context wiring, DOM-backed implementations, component rendering.
- Tests to add first:
  - Compile coverage for `PlatformEffects` consumers and no-op/default implementations.
  - Unit tests for default environment types such as `ColorMode`.
- Acceptance criteria:
  - Shared provider/platform interfaces exist in a crate that downstream code can depend on.
  - Effect closures no longer need to assume direct DOM access.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record whether the blocker is in `ars-core`, `ars-i18n`, or `ars-dom`.

### [#37](https://github.com/fogodev/ars-ui/issues/37): Implement `FormContext`, field descriptors, `FieldCtx`, and hidden-input helpers

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#17`
- Spec refs:
  - `spec/foundation/07-forms.md#5-form-context`
  - `spec/foundation/07-forms.md#6-field-association-ids-for-aria`
  - `spec/foundation/07-forms.md#7-hidden-inputs-for-form-submission`
  - `spec/foundation/07-forms.md#126-fieldctx-shared-context-for-child-fields`
- Goal: extend `ars-forms` from validation primitives into the shared field/form context layer expected by form-related utilities.
- Out of scope: adapter hooks, submit lifecycle state machine, adapter components.
- Tests to add first:
  - Unit tests for field registration order and field association helpers.
  - Unit tests for hidden-input serialization, including multi-value and disabled/form-associated cases.
  - Unit tests for server-error injection and field-level clearing behavior.
- Acceptance criteria:
  - `FormContext` and related field metadata types exist with spec-defined behavior.
  - Hidden-input helpers cover the common single/multi-value cases used by downstream components.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and name the missing dependency or unresolved forms-spec question.

### [#38](https://github.com/fogodev/ars-ui/issues/38): Implement the `form_submit` machine and focus-on-error helpers

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`, `#32`, `#33`, `#37`
- Spec refs:
  - `spec/foundation/07-forms.md#8-form-submit-state-machine`
  - `spec/foundation/07-forms.md#81-server-side-validation-error-sync-pattern`
  - `spec/foundation/07-forms.md#9-focus-management-on-submit-error`
- Goal: add the spec-defined form submission state machine and shared invalid-field focus helpers.
- Out of scope: Leptos/Dioxus hook APIs, concrete DOM focus calls, end-user form components.
- Tests to add first:
  - Unit tests for submit state transitions, validation failure, submission success/failure, and reset behavior.
  - Unit tests for `SetServerErrors` and first-invalid-field selection order.
- Acceptance criteria:
  - `ars-forms` exposes the submission lifecycle contract required by the `Form` utility.
  - Server-side validation error sync is part of the shared forms layer, not an adapter-only behavior.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record whether the blocker is missing `FormContext` or missing core attr/connect types.

### [#39](https://github.com/fogodev/ars-ui/issues/39): Implement DOM focus query and focus-scope support primitives in `ars-dom`

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Depends on: `#36`, `#18`
- Spec refs:
  - `spec/foundation/11-dom-utilities.md#3-focus-utilities`
  - `spec/foundation/11-dom-utilities.md#31-element-querying`
  - `spec/foundation/11-dom-utilities.md#32-focus-management`
  - `spec/foundation/11-dom-utilities.md#33-focusscope-implementation`
- Goal: implement the focus and focus-scope DOM utilities that the platform effects layer and `FocusScope` utility depend on.
- Out of scope: overlay scroll locking and positioning engine work.
- Tests to add first:
  - Unit tests for pure wrapper logic and fallback ordering where the code does not require a live DOM.
  - Web-targeted smoke tests or compile coverage for DOM query/focus entry points.
- Acceptance criteria:
  - `ars-dom` exposes the focus primitives needed by `PlatformEffects`.
  - Focus restoration and containment behavior exist as shared DOM utilities rather than adapter-local code.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and note whether the blocker is DOM test infrastructure or missing provider/platform contracts.

### [#40](https://github.com/fogodev/ars-ui/issues/40): Implement reference-counted scroll locking in `ars-dom`

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#18`
- Spec refs:
  - `spec/foundation/01-architecture.md#25-scrolllockmanager-ars-dom`
  - `spec/foundation/11-dom-utilities.md#52-scrolllockmanager-reference-counted`
  - `spec/foundation/11-dom-utilities.md#53-low-level-api-acquirerelease-with-depth-counter`
- Goal: replace the current shell with the reference-counted scroll lock manager required by overlay work.
- Out of scope: overlay machines and component-level integration.
- Tests to add first:
  - Unit tests for depth counting, duplicate owner protection, and restoration on last unlock.
  - Unit tests for public alias behavior (`prevent_scroll`, `restore_scroll`).
- Acceptance criteria:
  - `ars-dom` exposes the scroll locking API defined by the architecture and DOM specs.
  - Nested overlay locking semantics are covered without relying on component code.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and record the exact runtime or platform constraint.

### [#41](https://github.com/fogodev/ars-ui/issues/41): Strengthen `ars-interactions` attribute composition for composed utilities

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#31`, `#32`, `#16`
- Spec refs:
  - `spec/foundation/05-interactions.md#81-the-composition-problem`
  - `spec/foundation/05-interactions.md#82-merge_attrs`
  - `spec/components/utility/as-child.md#32-aria-attribute-merge-rules`
- Goal: bring attribute composition in line with the token-aware merge behavior expected by `AsChild`, `Button`, and related utilities.
- Out of scope: framework event-handler composition in adapters.
- Tests to add first:
  - Unit tests for merge precedence, token-list concatenation, and ARIA ID-list behavior.
- Acceptance criteria:
  - Shared interaction composition no longer does naive overwrite-only merging.
  - Downstream `as_child` and composed-control work has a spec-compliant shared merge base.
- Spec impact: `No spec change required`.
- Board update rule: if blocked, keep `Status` in `Todo` and note whether the blocker is in core attr modeling or in the interaction spec itself.

## Corrected Execution Order

The corrected order for near-term work is:

1. `#31` — typed connect primitives in `ars-core`
2. `#32` — spec-compliant `AttrMap` and style strategy
3. `#33` — `ars-a11y` ARIA bridge onto typed attrs
4. `#34` — `ars-a11y` role and state helpers
5. `#35` — anatomy/derive support
6. `#36` — provider and platform-effect shared contracts
7. `#41` — interaction composition depth
8. `#37` — forms context, field association, and hidden inputs
9. `#38` — form submit machine
10. `#39` — DOM focus and focus-scope primitives
11. `#40` — DOM scroll locking
12. Revisit `#24` only after the utility-slice prerequisites above exist as issue-backed work

This order intentionally front-loads the contracts that utility specs already import implicitly, then brings back slice decomposition only when those prerequisites are visible in the backlog.

## Concrete Next Task

The next unblocked, high-leverage implementation task after [#31](https://github.com/fogodev/ars-ui/issues/31) is:

**[#32](https://github.com/fogodev/ars-ui/issues/32): Implement spec-compliant `AttrMap`, `AttrValue`, `UserAttrs`, and `StyleStrategy`**.

Why this is next:

- It is the direct follow-up to `#31`, which introduced the typed connect enums.
- It unlocks `#33`, `#35`, and `#41`, all of which depend on the typed attribute container.
- It removes the biggest remaining source of spec/code mismatch in the shared connect layer, since the current specs already assume typed `AttrMap` behavior everywhere.

## Status of `#24`

- Keep `#24` open but deferred.
- Do not move it to `In Progress`.
- Treat this audit as the prerequisite planning artifact for replacing `#24` with the missing foundation cards above.
