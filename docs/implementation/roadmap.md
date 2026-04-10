# Roadmap

## Objective

Build `ars-ui` from a spec-only workspace into an implementation workspace with:

- architecture-aligned crates
- stable core contracts
- reusable subsystem primitives
- framework-agnostic test harnesses
- the full agnostic component layer (111 components across 9 categories)

## Phase Order

### Phase 0: Workspace bootstrap

Outcome:

- the workspace includes the architecture-defined crates
- each crate compiles with a minimal public surface
- release/profile defaults match the architecture baseline

Exit criteria:

- `cargo check --workspace` passes
- `cargo test --workspace` passes for bootstrap tests

### Phase 1: Core contract lock

Outcome:

- `ars-core` defines the minimum contract for `Machine`, `Service`, `TransitionPlan`, `PendingEffect`, `Bindable`, `ConnectApi`, and `AttrMap`
- `ars-derive` exposes the initial derive surface required to unblock anatomy and ID-related work

Exit criteria:

- unit tests cover the initial service runtime and controlled/uncontrolled behavior
- downstream crates compile against the shared contract without redefining local variants

### Phase 2: Cross-cutting subsystem base

Outcome:

- `ars-a11y`, `ars-interactions`, `ars-forms`, and `ars-dom` provide the shared primitives needed by the first utility slice

Exit criteria:

- each subsystem has a bounded set of unit and integration tests
- adapter crates consume shared primitives instead of copy-pasting framework-local logic

### Phase 3: Testing platform

Outcome:

- `ars-test-harness`, `ars-test-harness-leptos`, and `ars-test-harness-dioxus` expose a unified adapter testing entry point
- CI runs unit, integration, and adapter suites separately
- ARIA assertion helpers are available for all component tests
- `insta` snapshot infrastructure is wired into CI
- Adapter parity types (`ParityTestCase`, `InteractionTestCase`) enable cross-adapter testing

Exit criteria:

- test-harness API is stable enough for the first component slice
- CI failures identify the failing tier
- `ars-core/src/test_helpers.rs` exports 35+ ARIA assertion functions
- `insta` snapshot tests compile and CI rejects unapproved changes
- Both adapter backends can mount, query, and interact with components

Status (2026-04-10): Phase 3 crate shells and CI tier split are done (#19, #20). The full harness API, ARIA helpers, snapshot setup, adapter backends, parity types, CI enforcement, mock infrastructure, and nightly pipeline remain as 11 open tasks (#178–#188, 34 pts). See [Epic #7](https://github.com/fogodev/ars-ui/issues/7).

#### Adapter foundation audit (2026-04-10)

An audit of Epic #8 (Leptos adapter) revealed that the original 3 tasks (#22, #55, #105) covered ~40% of the foundational spec sections in `08-adapter-leptos.md`. Two new tasks were added to close the gaps before component work begins:

- [#190](https://github.com/fogodev/ars-ui/issues/190) — ArsProvider context, reactive props, controlled value helper (5 pts)
- [#191](https://github.com/fogodev/ars-ui/issues/191) — emit/emit_map, event mapping, nonce CSS collector, safe event listeners (3 pts)

A symmetric audit of Epic #9 (Dioxus adapter) confirmed the same gaps plus Dioxus-unique sections. Five new tasks were added (16 pts):

- [#193](https://github.com/fogodev/ars-ui/issues/193) — ArsProvider context, reactive props, controlled value helper (5 pts, symmetric with #190)
- [#194](https://github.com/fogodev/ars-ui/issues/194) — emit/emit_map, event mapping, nonce CSS collector, safe event listeners (3 pts, symmetric with #191)
- [#195](https://github.com/fogodev/ars-ui/issues/195) — DioxusPlatform trait, WebPlatform, DesktopPlatform, NullPlatform, use_platform() (3 pts, Dioxus-unique)
- [#196](https://github.com/fogodev/ars-ui/issues/196) — SSR Hydration: HydrationSnapshot, FocusScope hydration safety (3 pts)
- [#197](https://github.com/fogodev/ars-ui/issues/197) — ArsErrorBoundary component (2 pts)

See `foundation-completion-roadmap.md` for full task details and `foundation-gap-audit.md` for the gap matrix.

### Phase 4: Agnostic utility components

Scope:

All 26 utility components defined in `spec/components/utility/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (17):** AsChild, ArsProvider, ClientOnly, Dismissable, DownloadTrigger, Field, Fieldset, FocusRing, Form, Group, Heading, Highlight, Keyboard, Landmark, Separator, VisuallyHidden, ZIndexAllocator

**Stateful (9):** ActionGroup, Button, DropZone, FocusScope, LiveRegion, Swap, Toggle, ToggleButton, ToggleGroup

Decomposed into 20 tasks (64 story points) organized in 5 dependency waves. See [Epic #10](https://github.com/fogodev/ars-ui/issues/10) for the full task breakdown with sub-issues.

Exit criteria:

- all 26 utility components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

Status (2026-04-10): Epic repurposed from "First utility slice" (11 components) to cover all 26 agnostic utility components. Issue #24 (decomposition card) closed as superseded. Twenty new task issues (#199–#218) created as sub-issues of Epic #10.

### Phase 5: Agnostic layout components

Scope:

All 11 layout components defined in `spec/components/layout/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (5):** AspectRatio, Center, Frame, Grid, Stack

**Stateful (6):** Carousel, Collapsible, Portal, ScrollArea, Splitter, Toolbar

Decomposed into 8 tasks (31 story points) in a single wave — no intra-epic dependencies. See [Epic #226](https://github.com/fogodev/ars-ui/issues/226) for the full task breakdown with sub-issues.

Exit criteria:

- all 11 layout components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- CSS custom property hooks match the spec appendix for each component
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

Status (2026-04-10): Epic #226 created with 8 sub-issue tasks (#270–#281).

### Phase 6: Agnostic input components

Scope:

All 14 input components defined in `spec/components/input/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (1):** FileTrigger

**Stateful (11):** Checkbox, CheckboxGroup, Editable, NumberInput, PasswordInput, PinInput, RadioGroup, SearchInput, Switch, TextField, Textarea

**Complex (2):** Slider, RangeSlider

Decomposed into 12 tasks (48 story points) organized in 2 dependency waves. See [Epic #220](https://github.com/fogodev/ars-ui/issues/220) for the full task breakdown with sub-issues.

Exit criteria:

- all 14 input components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- form integration (hidden inputs, validation, field context) matches spec
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

Status (2026-04-10): Epic #220 created with 12 sub-issue tasks (#228–#251).

### Phase 7: Agnostic data-display components

Scope:

All 11 data-display components defined in `spec/components/data-display/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (4):** Badge, Meter, Skeleton, Stat

**Stateful (6):** Avatar, GridList, Marquee, Progress, RatingGroup, TagGroup

**Complex (1):** Table

Decomposed into 9 tasks (40 story points) organized in 3 dependency waves. See [Epic #225](https://github.com/fogodev/ars-ui/issues/225) for the full task breakdown with sub-issues.

Exit criteria:

- all 11 data-display components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- collection-dependent components (GridList, TagGroup, Table) use `ars-collections` for item management
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: GridList, TagGroup, and Table depend on ars-collections (Epic #53).

Status (2026-04-10): Epic #225 created with 9 sub-issue tasks (#266–#286).

### Phase 8: Agnostic overlay components

Scope:

All 10 overlay components defined in `spec/components/overlay/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateful (4):** AlertDialog, Popover, Presence, Tooltip

**Complex (6):** Dialog, Drawer, FloatingPanel, HoverCard, Toast, Tour

Decomposed into 10 tasks (50 story points) organized in 4 dependency waves. See [Epic #222](https://github.com/fogodev/ars-ui/issues/222) for the full task breakdown with sub-issues.

Exit criteria:

- all 10 overlay components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- z-index allocation uses `next_z_index()` from ars-dom for overlay components
- focus trapping implemented for Dialog and AlertDialog
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: z-index-stacking (#68, completed). Dialog/AlertDialog depend on FocusScope for focus trapping.

Status (2026-04-10): Epic #222 created with 10 sub-issue tasks (#238–#265).

### Phase 9: Agnostic navigation components

Scope:

All 8 navigation components defined in `spec/components/navigation/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (1):** Breadcrumbs

**Stateful (4):** Accordion, Link, Pagination, Steps

**Complex (3):** NavigationMenu, Tabs, TreeView

Decomposed into 7 tasks (33 story points) organized in 3 dependency waves. See [Epic #223](https://github.com/fogodev/ars-ui/issues/223) for the full task breakdown with sub-issues.

Exit criteria:

- all 8 navigation components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- TreeView uses `TreeCollection` from ars-collections for hierarchical navigation
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: TreeView depends on ars-collections TreeCollection (#83).

Status (2026-04-10): Epic #223 created with 7 sub-issue tasks (#247–#267).

### Phase 10: Agnostic selection components

Scope:

All 9 selection components defined in `spec/components/selection/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateful (2):** Autocomplete, SegmentGroup

**Complex (7):** Combobox, ContextMenu, Listbox, Menu, MenuBar, Select, TagsInput

Decomposed into 9 tasks (55 story points) organized in 4 dependency waves. See [Epic #221](https://github.com/fogodev/ars-ui/issues/221) for the full task breakdown with sub-issues.

Exit criteria:

- all 9 selection components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- collection-dependent components use `ars-collections` Collection trait for navigation and typeahead
- selection patterns match `shared/selection-patterns.md`
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: All except SegmentGroup depend on ars-collections (Epic #53).

Status (2026-04-10): Epic #221 created with 9 sub-issue tasks (#232–#255).

### Phase 11: Agnostic specialized components

Scope:

All 15 specialized components defined in `spec/components/specialized/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (3):** ColorSwatch, ContextualHelp, QrCode

**Stateful (10):** AngleSlider, Clipboard, ColorArea, ColorField, ColorSlider, ColorSwatchPicker, ColorWheel, ImageCropper, SignaturePad, Timer

**Complex (2):** ColorPicker, FileUpload

Decomposed into 11 tasks (55 story points) organized in 4 dependency waves. See [Epic #227](https://github.com/fogodev/ars-ui/issues/227) for the full task breakdown with sub-issues.

Exit criteria:

- all 15 specialized components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- color components share `ColorValue` type; ColorPicker composes all color primitives
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: FileUpload depends on DnD interactions (#159–#161).

Status (2026-04-10): Epic #227 created with 11 sub-issue tasks (#288–#301).

### Phase 12: Agnostic date-time components

Scope:

All 8 date-time components defined in `spec/components/date-time/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateful (5):** DateField, DateRangeField, DateRangePicker, RangeCalendar, TimeField

**Complex (3):** Calendar, DatePicker, DateTimePicker

Decomposed into 8 tasks (47 story points) organized in 4 dependency waves. See [Epic #224](https://github.com/fogodev/ars-ui/issues/224) for the full task breakdown with sub-issues.

Exit criteria:

- all 8 date-time components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- calendar correctly uses locale-aware first-day-of-week via WeekInfo
- date validation respects per-calendar month/day bounds via IcuProvider
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

External deps: All depend on ars-i18n (Epic #54) for CalendarDate, DateFormatter, WeekInfo. Date fields and pickers depend on ars-forms (Epic #5) for form integration.

Status (2026-04-10): Epic #224 created with 8 sub-issue tasks (#262–#292).

## Spec synchronization rules

- Each implementation task must declare `Spec impact`.
- If the implementation proves the spec wrong or incomplete, update the spec in the same task.
- Shared abstraction changes go into `spec/foundation/` or `spec/shared/`.
- Adapter-specific realization belongs in `spec/foundation/08-adapter-leptos.md`, `spec/foundation/09-adapter-dioxus.md`, and the per-component adapter specs.
- Adapter code must not become the only authoritative explanation for future framework ports.
