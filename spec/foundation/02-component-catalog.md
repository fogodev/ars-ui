# Component Catalog

## 1. Overview

ars-ui targets **111 components (including 1 internal utility: Dismissable) and 6 format utilities (117 total)** covering the union of Ark UI and React Aria feature sets. Components are organized into categories and assigned priority tiers that respect dependency ordering.

### 1.1 Priority Tiers

| Tier                | Description                               | Target    |
| ------------------- | ----------------------------------------- | --------- |
| **P0 — Foundation** | Core primitives everything else builds on | v0.1      |
| **P1 — Essential**  | Most common UI patterns                   | v0.2      |
| **P2 — Complete**   | Full-featured library                     | v0.3      |
| **P3 — Extended**   | Specialized or niche components           | post-v1.0 |

## 2. Input Components

| Component         | Ark UI | React Aria | Priority | Description                                                                                                                                                                                                                                                                                                                  |
| ----------------- | :----: | :--------: | :------: | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Checkbox**      |   Y    |     Y      |    P0    | Toggle between checked/unchecked/indeterminate                                                                                                                                                                                                                                                                               |
| **CheckboxGroup** |   Y    |     Y      |    P0    | Group of checkboxes with shared state, name, and indeterminate computation                                                                                                                                                                                                                                                   |
| **RadioGroup**    |   Y    |     Y      |    P0    | Mutually exclusive option group                                                                                                                                                                                                                                                                                              |
| **Switch**        |   Y    |     Y      |    P0    | On/off toggle control                                                                                                                                                                                                                                                                                                        |
| **TextField**     |   —    |     Y      |    P0    | Text input and textarea                                                                                                                                                                                                                                                                                                      |
| **Textarea**      |   —    |     Y      |    P0    | Multi-line text input with auto-grow and character count                                                                                                                                                                                                                                                                     |
| **NumberInput**   |   Y    |     Y      |    P1    | Numeric input with increment/decrement, locale-aware                                                                                                                                                                                                                                                                         |
| **Slider**        |   Y    |     Y      |    P1    | Single and range value slider                                                                                                                                                                                                                                                                                                |
| **RangeSlider**   |   —    |     Y      |    P1    | Dual-thumb slider for selecting a value range. Has separate start/end value tracking with `start <= end` validation, dual-thumb independent focus states, and optional per-thumb disabled. References Slider spec; deviations: two `Bindable<f64>` values, `ThumbFocus(Start\|End)` state, range validation on every change. |
| **SearchInput**   |   —    |     Y      |    P1    | Search input with clear button. Specialized text input with `role="searchbox"`, Idle/Focused/Searching states (extends TextField's Idle/Focused with a `Searching` state), clear button, and submit handling. Uses Collection trait for Autocomplete integration. Full spec in `components/input/search-input.md`.           |
| **PinInput**      |   Y    |     —      |    P2    | OTP-style single-character inputs                                                                                                                                                                                                                                                                                            |
| **PasswordInput** |   Y    |     —      |    P2    | Text input with show/hide toggle                                                                                                                                                                                                                                                                                             |
| **Editable**      |   Y    |     —      |    P2    | Inline text that switches to input                                                                                                                                                                                                                                                                                           |
| **FileTrigger**   |   —    |     Y      |    P1    | Click-to-browse file selection trigger. Stateless utility — Props: `accept: Option<String>`, `multiple: bool`, `directory: bool`. Adapter-level: `on_select: Callback<Vec<File>>`. Full spec in `components/input/file-trigger.md`.                                                                                          |

## 3. Selection Components

| Component        | Ark UI | React Aria | Priority | Description                                                                                                                                                                                                        |
| ---------------- | :----: | :--------: | :------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Select**       |   Y    |     Y      |    P0    | Dropdown single/multi-select                                                                                                                                                                                       |
| **Menu**         |   Y    |     Y      |    P0    | Dropdown menu with items, groups, submenus                                                                                                                                                                         |
| **Listbox**      |   Y    |     Y      |    P1    | Inline selectable list                                                                                                                                                                                             |
| **Combobox**     |   Y    |     Y      |    P1    | Filterable select with text input                                                                                                                                                                                  |
| **Autocomplete** |   —    |     —      |    P2    | Searchable menu / command palette. Pairs SearchInput with Menu using FilteredCollection. `role="combobox"` on input + `role="menu"` on results. Fully specified in `components/selection/autocomplete.md`.         |
| **ContextMenu**  |  Y\*   |     —      |    P0    | Right-click / long-press context menu. \*Ark-UI implements as Menu with context trigger. (see components/selection/context-menu.md)                                                                                |
| **SegmentGroup** |   Y    |     —      |    P1    | Mutually exclusive segmented control (toggle between segments). Similar to ToggleGroup but with connected visual segments and `role="radiogroup"` semantics. Specified in `components/selection/segment-group.md`. |
| **MenuBar**      |   —    |     Y      |    P2    | Horizontal menu bar with keyboard navigation (`role="menubar"`)                                                                                                                                                    |
| **TagsInput**    |   Y    |     —      |    P2    | Multi-value token input                                                                                                                                                                                            |

## 4. Overlay Components

| Component         | Ark UI | React Aria | Priority | Description                                                                                                               |
| ----------------- | :----: | :--------: | :------: | ------------------------------------------------------------------------------------------------------------------------- |
| **Dialog**        |   Y    |     Y      |    P0    | Modal and non-modal dialog window                                                                                         |
| **Popover**       |   Y    |     Y      |    P0    | Floating content anchored to trigger                                                                                      |
| **Tooltip**       |   Y    |     Y      |    P1    | Small informational popup on hover/focus                                                                                  |
| **Toast**         |   Y    |     Y      |    P1    | Auto-dismissing notification                                                                                              |
| **AlertDialog**   |   —    |     Y      |    P1    | Destructive action confirmation                                                                                           |
| **Drawer**        |  Y\*   |     —      |    P1    | Slide-in panel from screen edge. \*Ark-UI implements as Dialog variant with placement. (see components/overlay/drawer.md) |
| **HoverCard**     |   Y    |     —      |    P2    | Rich content preview on hover                                                                                             |
| **FloatingPanel** |   Y    |     —      |    P3    | Draggable/resizable floating window. Specified in `components/overlay/floating-panel.md`.                                 |
| **Tour**          |   Y    |     —      |    P3    | Guided step-by-step walkthrough. Specified in `components/overlay/tour.md`.                                               |
| **Presence**      |   Y    |     —      |    P0    | Mount/unmount with animation support                                                                                      |

## 5. Navigation Components

| Component          | Ark UI |     React Aria      | Priority | Description                                                                                                                  |
| ------------------ | :----: | :-----------------: | :------: | ---------------------------------------------------------------------------------------------------------------------------- |
| **Tabs**           |   Y    |          Y          |    P0    | Tabbed content panels                                                                                                        |
| **Link**           |   —    |          Y          |    P0    | Accessible link with router integration                                                                                      |
| **Accordion**      |   Y    | Y (DisclosureGroup) |    P1    | Collapsible content sections                                                                                                 |
| **Breadcrumbs**    |   Y    |          Y          |    P2    | Path navigation                                                                                                              |
| **TreeView**       |   Y    |          Y          |    P2    | Hierarchical expandable tree                                                                                                 |
| **Pagination**     |   Y    |          —          |    P2    | Page navigation controls                                                                                                     |
| **Steps**          |   Y    |          —          |    P2    | Multi-step workflow indicator. Full spec in `components/navigation/steps.md`.                                                |
| **NavigationMenu** |   —    |          —          |    P2    | Horizontal/vertical nav bar with hover-triggered dropdown submenus. Full spec in `components/navigation/navigation-menu.md`. |

## 6. Date and Time Components

| Component           | Ark UI | React Aria | Priority | Description                                  |
| ------------------- | :----: | :--------: | :------: | -------------------------------------------- |
| **Calendar**        |   —    |     Y      |    P1    | Calendar grid for date selection             |
| **DatePicker**      |   Y    |     Y      |    P1    | Date field + calendar popover                |
| **DateField**       |   —    |     Y      |    P2    | Segmented date input (month/day/year)        |
| **TimeField**       |   —    |     Y      |    P2    | Segmented time input (hour/minute/period)    |
| **DateRangePicker** |   —    |     Y      |    P2    | Two date fields + range calendar             |
| **DateRangeField**  |   —    |     Y      |    P2    | Inline two-field range input without popover |
| **RangeCalendar**   |   —    |     Y      |    P2    | See Calendar with `is_range: true` prop      |
| **DateTimePicker**  |   Y    |     —      |    P2    | Combined date and time picker                |

## 7. Data Display Components

| Component       | Ark UI | React Aria | Priority | Description                                                                                                         |
| --------------- | :----: | :--------: | :------: | ------------------------------------------------------------------------------------------------------------------- |
| **Table**       |   —    |     Y      |    P1    | Sortable, selectable data table                                                                                     |
| **Progress**    |   Y    |     Y      |    P1    | Linear and circular progress indicator                                                                              |
| **Meter**       |   —    |     Y      |    P2    | Value within a known range                                                                                          |
| **Avatar**      |   Y    |     —      |    P2    | Image with fallback                                                                                                 |
| **Marquee**     |   Y    |     —      |    P3    | Scrolling text/content                                                                                              |
| **GridList**    |   —    |     Y      |    P2    | Keyboard-navigable grid of selectable items                                                                         |
| **Badge**       |   —    |     —      |    P2    | Dynamic count or status indicator                                                                                   |
| **Stat**        |   —    |     —      |    P2    | Statistic display with label, value, and change indicator                                                           |
| **Skeleton**    |   —    |     —      |    P2    | Loading placeholder with pulse/wave/shimmer animation variants. Specified in `components/data-display/skeleton.md`. |
| **RatingGroup** |   Y    |     —      |    P2    | Star/icon-based rating                                                                                              |
| **TagGroup**    |   —    |     Y      |    P2    | Group of removable tags (display, not input)                                                                        |

## 8. Layout Components

| Component       | Ark UI |   React Aria   | Priority | Description                                                               |
| --------------- | :----: | :------------: | :------: | ------------------------------------------------------------------------- |
| **Collapsible** |   Y    | Y (Disclosure) |    P1    | Expandable/collapsible container                                          |
| **Splitter**    |   Y    |       —        |    P2    | Resizable split-pane container                                            |
| **ScrollArea**  |   Y    |       —        |    P3    | Custom-styled scrollable container                                        |
| **Toolbar**     |   —    |       Y        |    P2    | Grouping of action buttons. See `components/layout/toolbar.md`.           |
| **AspectRatio** |   —    |       —        |    P3    | Maintain aspect ratio container                                           |
| **Frame**       |   —    |       —        |    P3    | Iframe wrapper with sandboxing and responsive sizing                      |
| **Carousel**    |   Y    |       —        |    P2    | Sliding content navigation. Specified in `components/layout/carousel.md`. |
| **Portal**      |   Y    |       Y        |    P0    | Render children in a different DOM subtree                                |
| **Stack**       |   —    |       —        |    P2    | Flex layout primitive. Specified in `components/layout/stack.md`.         |
| **Center**      |   —    |       —        |    P2    | Centering layout primitive. Specified in `components/layout/center.md`.   |
| **Grid**        |   —    |       —        |    P2    | Grid layout primitive. Specified in `components/layout/grid.md`.          |

## 9. Specialized Components

| Component             | Ark UI | React Aria | Priority | Description                                                                                                                                                                                                                                                                                                                                                                                        |
| --------------------- | :----: | :--------: | :------: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ColorPicker**       |   Y    |     Y      |    P2    | Color selection with multiple color spaces. Supported spaces: HSL, HSB, RGB, HEX (minimum). Internal `ColorValue` stored as HSL+alpha; conversions via `to_rgb()`, `to_hex()`, `to_hsb()`, `from_rgb()`, `from_hex()`. `ColorSpace` enum controls picker UI; `ColorFormat` enum controls text display. Space switching re-maps area/slider channels. See `components/specialized/color-picker.md`. |
| **ColorArea**         |   Y    |     Y      |    P2    | 2D gradient area for saturation/brightness selection (ColorPicker sub-component). Specified in `components/specialized/color-area.md`.                                                                                                                                                                                                                                                             |
| **ColorSlider**       |   Y    |     Y      |    P2    | 1D color channel slider (ColorPicker sub-component). Specified in `components/specialized/color-slider.md`.                                                                                                                                                                                                                                                                                        |
| **ColorField**        |   —    |     Y      |    P2    | Text input for typing/editing color values with format parsing                                                                                                                                                                                                                                                                                                                                     |
| **ColorWheel**        |   —    |     Y      |    P1    | Circular hue selector with drag-to-rotate thumb                                                                                                                                                                                                                                                                                                                                                    |
| **ColorSwatch**       |   Y    |     Y      |    P2    | Single color preview tile. Specified in `components/specialized/color-swatch.md`.                                                                                                                                                                                                                                                                                                                  |
| **ColorSwatchPicker** |   Y    |     Y      |    P2    | Selectable grid of color swatches. Specified in `components/specialized/color-swatch-picker.md`.                                                                                                                                                                                                                                                                                                   |
| **ImageCropper**      |   Y    |     —      |    P3    | Image region selection and cropping                                                                                                                                                                                                                                                                                                                                                                |
| **AngleSlider**       |   Y    |     —      |    P3    | Circular slider for angle values                                                                                                                                                                                                                                                                                                                                                                   |
| **ContextualHelp**    |   —    |     Y      |    P2    | Popover with contextual help content triggered by a help icon. Specified in `components/specialized/contextual-help.md`.                                                                                                                                                                                                                                                                           |
| **FileUpload**        |   Y    |     Y      |    P2    | Drag-and-drop or click-to-browse file selection                                                                                                                                                                                                                                                                                                                                                    |
| **SignaturePad**      |   Y    |     —      |    P3    | Canvas-based handwritten signature                                                                                                                                                                                                                                                                                                                                                                 |
| **Clipboard**         |   Y    |     —      |    P1    | Copy-to-clipboard utility with state machine tracking copy lifecycle. Specified in `components/specialized/clipboard.md`.                                                                                                                                                                                                                                                                          |
| **QRCode**            |   Y    |     —      |    P3    | QR code generation                                                                                                                                                                                                                                                                                                                                                                                 |
| **Timer**             |   Y    |     —      |    P3    | Countdown/count-up timer                                                                                                                                                                                                                                                                                                                                                                           |

## 10. Utility Components

| Component                    | Ark UI | React Aria | Priority | Description                                                                                                                                                                                                                                                                                 |
| ---------------------------- | :----: | :--------: | :------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Button**                   |   —    |     Y      |    P0    | Accessible button with press handling                                                                                                                                                                                                                                                       |
| **ToggleButton**             |   —    |     Y      |    P0    | Button with pressed/unpressed state                                                                                                                                                                                                                                                         |
| **ToggleGroup**              |   Y    |     Y      |    P1    | Group of toggle buttons (single/multi)                                                                                                                                                                                                                                                      |
| **VisuallyHidden**           |   —    |     Y      |    P0    | Content visible only to screen readers                                                                                                                                                                                                                                                      |
| **Separator**                |   —    |     Y      |    P1    | Visual/semantic divider                                                                                                                                                                                                                                                                     |
| **FocusScope**               |   Y    |     Y      |    P0    | Focus containment and restoration (Ark UI: Focus Trap)                                                                                                                                                                                                                                      |
| **FocusRing**                |   —    |     Y      |    P0    | Consistent keyboard focus indicator styling                                                                                                                                                                                                                                                 |
| **LiveRegion**               |   —    |     Y      |    P0    | Screen reader announcement utility (`aria-live`). Spec name: LiveRegion. React Aria equivalent: LiveAnnouncer.                                                                                                                                                                              |
| **DropZone**                 |   —    |     Y      |    P2    | Standalone drag-and-drop target area                                                                                                                                                                                                                                                        |
| **Swap**                     |   Y    |     —      |    P2    | Toggle between two visual states (e.g., sun/moon icon). Specified in `components/utility/swap.md`.                                                                                                                                                                                          |
| **DownloadTrigger**          |   Y    |     —      |    P2    | Declarative download trigger via `<a download>`. Specified in `components/utility/download-trigger.md`.                                                                                                                                                                                     |
| **Keyboard**                 |   —    |     Y      |    P1    | Renders keyboard shortcut text in `<kbd>` elements with optional platform-aware formatting                                                                                                                                                                                                  |
| **Landmark**                 |   —    |     Y      |    P1    | Semantic landmark regions (`<main>`, `<nav>`, `<aside>`, etc.) with ARIA role/label wiring. Specified in `components/utility/landmark.md`.                                                                                                                                                  |
| **Heading**                  |   —    |     Y      |    P1    | Heading with automatic level management via HeadingLevelProvider. Specified in `components/utility/heading.md`.                                                                                                                                                                             |
| **ClientOnly**               |   Y    |     —      |    P1    | Renders children only on the client (SSR guard). Specified in `components/utility/client-only.md`.                                                                                                                                                                                          |
| **ActionGroup**              |   —    |     Y      |    P2    | Grouped set of action buttons with overflow menu. Specified in `components/utility/action-group.md`.                                                                                                                                                                                        |
| **Highlight**                |   Y    |     —      |    P2    | Text substring highlighting for search results. Specified in `components/utility/highlight.md`.                                                                                                                                                                                             |
| **Dismissable** _(internal)_ |   —    |     Y      |    P0    | **Internal utility** — not part of the public component API. Dismisses overlay content on outside click or Escape. Used internally by Dialog, Popover, Menu. Specified in `components/utility/dismissable.md`. Consumers should not use directly; overlay components compose it internally. |
| **AsChild**                  |   Y    |     —      |    P0    | Primitives composition utility — render component as a child element                                                                                                                                                                                                                        |
| **ArsProvider**              |   Y    |     —      |    P0    | Single root provider supplying shared configuration (locale, direction, color mode, disabled/read-only cascades, ID prefix, portal/focus scope boundaries) to all descendant components                                                                                                     |
| **ZIndexAllocator**          |   —    |     —      |    P1    | Dynamic z-index allocation for stacking context management                                                                                                                                                                                                                                  |
| **Toggle**                   |   Y    |     —      |    P1    | Stateful toggle primitive with on/off state. Simpler than ToggleButton — no press interaction, just state management. Specified in `components/utility/toggle.md`.                                                                                                                          |
| **Field**                    |   Y    |     Y      |    P1    | Wrapper associating label, input, description, and error message via ARIA linkage. Specified in `components/utility/field.md` and `07-forms.md` §13.                                                                                                                                        |
| **Fieldset**                 |   Y    |     —      |    P1    | Groups related form fields with `<fieldset>`/`<legend>` semantics and shared disabled/error state propagation. Specified in `components/utility/fieldset.md`.                                                                                                                               |
| **Form**                     |   Y    |     Y      |    P0    | Form submission wrapper with validation lifecycle, hidden input collection, and server error handling. Specified in `components/utility/form.md`.                                                                                                                                           |
| **Group**                    |   —    |     Y      |    P1    | General-purpose grouping with `disabled`/`invalid`/`read_only` propagation via context and `role="group"`. Specified in `components/utility/group.md`.                                                                                                                                      |

## 11. Format Utilities

| Utility                | Ark UI | React Aria | Priority | Description                    |
| ---------------------- | :----: | :--------: | :------: | ------------------------------ |
| **FormatNumber**       |   Y    |     Y      |    P1    | Locale-aware number formatting |
| **FormatDate**         |   —    |     Y      |    P1    | Locale-aware date formatting   |
| **FormatTime**         |   Y    |     —      |    P1    | Locale-aware time formatting   |
| **FormatRelativeTime** |   Y    |     Y      |    P2    | Relative time ("3 days ago")   |
| **FormatByte**         |   Y    |     —      |    P2    | Byte value formatting          |
| **FormatList**         |   —    |     Y      |    P2    | Locale-aware list formatting   |

## 12. Cross-Component Dependencies

```diagram
                  Button
                    |
    +-----------+---------+-------+---------+
    |           |         |       |         |
 Checkbox  RadioGroup  Switch  Toggle  ToggleButton
                                  |
                              ToggleGroup

    FocusScope ──> Dialog ──> AlertDialog
                      |
    Presence     Popover ──> HoverCard
        |         |    \
    Collapsible   |     Tooltip
        |         |
    Accordion    Menu ──> ContextMenu
                  |
              +---+---+
              |       |
           Select  Combobox
              |       |
           Listbox    |
              |       |
          Collection Trait ──> Table
              |                  |
           TreeView          GridList
              |
           TagGroup / TagsInput

    Calendar ──> DatePicker ──> DateRangePicker
        |            |
        |        DateField ──> DateRangeField
        |
    RangeCalendar

    Slider ──> ColorPicker (ColorSlider, ColorArea)
        |           |
    NumberInput     +──> ColorSwatch ──> ColorSwatchPicker
        |
    AngleSlider

    DropZone (standalone drop target, reused by FileUpload)

    Positioning Engine ──> Popover, Tooltip, Menu, Select, Combobox, HoverCard, DatePicker
```

> This diagram shows major cross-component dependency chains. Components not shown have no significant inter-component dependencies beyond their foundation deps.

## 13. Implementation Prerequisites for P0 Overlays

P0 overlay components (Dialog, Popover) depend on several foundational subsystems that must be implemented first:

- **Presence** — mount/unmount animation support (see `components/overlay/presence.md`)
- **FocusScope** — focus trapping and restoration for modal content
- **Positioning engine** — `compute_position()` + `auto_update()` from `ars-dom` (11-dom-utilities.md §2)
- **Scroll locking** — `ScrollLockManager` from `ars-dom` (11-dom-utilities.md §5) for nested overlays, `prevent_scroll()` for simple cases
- **Z-index management** — stacking context strategy for nested overlays (portal-based)

These must be available before Dialog or Popover can be fully functional.

## 14. Implementation Roadmap

### 14.1 Phase 1: P0 Foundation (25 components including 4 a11y utilities)

Core infrastructure + foundational components:

1. `ars-core`: Machine trait, Service, Connect (ConnectApi trait), AttrMap, Bindable
2. `ars-a11y`: ARIA types, FocusScope, FocusRing, LiveRegion, VisuallyHidden (these are also public P0 components counted in the 25 total)
3. `ars-dom`: ID generation, focus utilities, positioning engine
4. `ars-interactions`: Press, Hover, Focus (basics)
5. Components: Button, ToggleButton, Checkbox, CheckboxGroup, RadioGroup, Switch, TextField, Textarea, Link
6. Components: Dialog, Popover, Tabs, Select, Menu, ContextMenu
7. Utilities: Presence, Portal, Dismissable, AsChild, ArsProvider, Form
8. Adapters: ars-leptos, ars-dioxus (basic wiring)

### 14.2 Phase 2: P1 Essential (32 components/utilities)

Common patterns + i18n foundation:

1. `ars-i18n`: Locale system, RTL, number formatting
2. `ars-collections`: Collection trait, selection model
3. `ars-forms`: Validation framework, field association
4. Components: NumberInput, Slider, RangeSlider, SearchInput, Combobox, Listbox
5. Components: Tooltip, Toast, AlertDialog, Drawer, Accordion, Collapsible
6. Components: Calendar, DatePicker, Table, Progress, ColorWheel
7. Utilities: ToggleGroup, Toggle, Separator, Keyboard, FormatNumber, FormatDate, FormatTime, Field, Fieldset, Landmark, Heading, ClientOnly, ZIndexAllocator, Group
8. Selection: SegmentGroup
9. Input: FileTrigger

### 14.3 Phase 3: P2 Complete (~46 components/utilities)

Full catalog:

1. `ars-i18n`: Calendar systems, date math, full i18n
2. `ars-interactions`: LongPress, Move, DnD
3. `ars-collections`: Virtualization, async loading
4. Components: PinInput, TagsInput, PasswordInput, Editable, FileUpload, RatingGroup, Autocomplete
5. Components: HoverCard, TagGroup, Breadcrumbs, TreeView, Pagination, Steps, Carousel, MenuBar
6. Components: DateField, TimeField, DateTimePicker, DateRangePicker, DateRangeField, RangeCalendar
7. Components: Meter, Avatar, Badge, Stat, Splitter, Stack, Center, Grid, Toolbar, ColorPicker, ColorArea, ColorSlider, ColorField, ColorSwatch, ColorSwatchPicker, Clipboard, GridList, DropZone, ContextualHelp, Skeleton
8. Utilities: FormatRelativeTime, FormatByte, FormatList, ActionGroup, Highlight, Swap, DownloadTrigger

### 14.4 Phase 4: P3 Extended

Post-v1.0 specialized components:

- FloatingPanel, Tour, QRCode, Timer, ScrollArea, SignaturePad, ImageCropper
- Marquee, AngleSlider, AspectRatio, Frame

---

> **Note:** The Steps component specification has been moved to `components/navigation/steps.md`.
> See the manifest for its full dependency listing.
