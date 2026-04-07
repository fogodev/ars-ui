# ars-ui

[![codecov](https://codecov.io/github/fogodev/ars-ui/graph/badge.svg?token=OE69AOOYHK)](https://codecov.io/github/fogodev/ars-ui)

A Rust-native, framework-agnostic UI component library built on state machines. Production-grade, accessible, and internationalized from the ground up.

## What is ars-ui?

**ars-ui** (A Rusty UI) is the first framework-agnostic Rust UI component library. Core component logic lives in pure Rust state machines with zero framework dependency. Thin adapter layers bridge these machines into [Leptos](https://leptos.dev/) and [Dioxus](https://dioxuslabs.com/), so you write component logic once and use it across frameworks.

The architecture draws from [Ark UI](https://ark-ui.com/) (state machine-driven components) and [React Aria](https://react-aria.adobe.com/) (deep accessibility and i18n rigor).

## Design Principles

- **Accessibility first** -- every component meets WCAG 2.1 Level AA minimum, following WAI-ARIA Authoring Practices
- **Framework-agnostic core** -- state machines in `ars-core` have zero framework imports; they produce abstract state transitions and DOM attribute maps
- **Type safety over runtime checks** -- state enums make invalid states unrepresentable; typed ARIA attributes prevent misuse at compile time
- **Headless by default** -- zero styling opinions; components emit data attributes (`data-ars-state`, `data-ars-part`) for CSS targeting
- **Internationalization built-in** -- RTL layouts, locale-aware formatting, and translatable accessibility strings out of the box
- **Incremental adoption** -- each component is independently usable; feature flags control what gets compiled

## Components

111 components across 9 categories.

### Input

| Component      | Description                                       |
| -------------- | ------------------------------------------------- |
| Checkbox       | Single boolean toggle with indeterminate support  |
| Checkbox Group | Grouped set of checkboxes with shared state       |
| Editable       | Inline-editable text with view/edit modes         |
| File Trigger   | Button that opens the native file picker          |
| Number Input   | Numeric input with increment/decrement controls   |
| Password Input | Text input with visibility toggle                 |
| Pin Input      | Multi-segment code entry (OTP, PIN)               |
| Radio Group    | Single-selection from a set of options            |
| Range Slider   | Dual-thumb slider for selecting a range           |
| Search Input   | Text input with search semantics and clear action |
| Slider         | Single-thumb value selection along a track        |
| Switch         | Binary toggle with on/off semantics               |
| Text Field     | Single-line text input                            |
| Textarea       | Multi-line text input                             |

### Selection

| Component     | Description                                     |
| ------------- | ----------------------------------------------- |
| Autocomplete  | Text input with filtered suggestion list        |
| Combobox      | Combined text input and dropdown selection      |
| Context Menu  | Right-click triggered menu                      |
| Listbox       | Scrollable list with single or multi-select     |
| Menu          | Action list triggered by a button               |
| Menu Bar      | Horizontal bar of menus (application-style)     |
| Segment Group | Mutually exclusive button group                 |
| Select        | Dropdown selection from a list of options       |
| Tags Input    | Multi-value input with tag creation and removal |

### Overlay

| Component      | Description                                             |
| -------------- | ------------------------------------------------------- |
| Alert Dialog   | Modal requiring acknowledgment before proceeding        |
| Dialog         | Modal or non-modal content overlay                      |
| Drawer         | Slide-in panel from a screen edge                       |
| Floating Panel | Draggable, resizable floating container                 |
| Hover Card     | Content preview shown on pointer hover                  |
| Popover        | Anchored content overlay triggered by a control         |
| Presence       | Manages mount/unmount transitions for animated elements |
| Toast          | Temporary notification messages                         |
| Tooltip        | Contextual information on hover or focus                |
| Tour           | Step-by-step guided walkthrough                         |

### Navigation

| Component       | Description                                        |
| --------------- | -------------------------------------------------- |
| Accordion       | Vertically stacked collapsible sections            |
| Breadcrumbs     | Hierarchical navigation trail                      |
| Link            | Navigation anchor with routing integration         |
| Navigation Menu | Complex navigation with nested submenus            |
| Pagination      | Page-level navigation controls                     |
| Steps           | Multi-step progress indicator and navigation       |
| Tabs            | Tabbed content panels with keyboard navigation     |
| Tree View       | Hierarchical expandable/collapsible node structure |

### Date & Time

| Component         | Description                                    |
| ----------------- | ---------------------------------------------- |
| Calendar          | Full month/year grid for date selection        |
| Date Field        | Segmented date input with keyboard editing     |
| Date Picker       | Calendar-backed date selection with text input |
| Date Range Field  | Segmented start/end date input                 |
| Date Range Picker | Calendar-backed range selection                |
| Date Time Picker  | Combined date and time selection               |
| Range Calendar    | Calendar grid for selecting date ranges        |
| Time Field        | Segmented time input with keyboard editing     |

### Data Display

| Component    | Description                                      |
| ------------ | ------------------------------------------------ |
| Avatar       | User or entity image with fallback               |
| Badge        | Small label for status or counts                 |
| Grid List    | Interactive grid of selectable items             |
| Marquee      | Scrolling content ticker                         |
| Meter        | Visual indicator of a value within a known range |
| Progress     | Determinate or indeterminate loading indicator   |
| Rating Group | Star/icon-based rating input                     |
| Skeleton     | Placeholder shapes for loading states            |
| Stat         | Key metric with label and optional trend         |
| Table        | Structured data grid with sorting and selection  |
| Tag Group    | Read-only collection of labeled tags             |

### Layout

| Component    | Description                                      |
| ------------ | ------------------------------------------------ |
| Aspect Ratio | Constrains children to a fixed aspect ratio      |
| Carousel     | Horizontally scrollable slide container          |
| Center       | Centers content horizontally and vertically      |
| Collapsible  | Expandable/collapsible content region            |
| Frame        | Fixed-ratio responsive container                 |
| Grid         | CSS grid layout helper                           |
| Portal       | Renders children into a different DOM subtree    |
| Scroll Area  | Custom-styled scrollable container               |
| Splitter     | Resizable split pane layout                      |
| Stack        | Vertical or horizontal flex layout               |
| Toolbar      | Grouped set of controls with keyboard navigation |

### Specialized

| Component           | Description                                               |
| ------------------- | --------------------------------------------------------- |
| Angle Slider        | Circular slider for angle/rotation values                 |
| Clipboard           | Copy-to-clipboard with feedback                           |
| Color Area          | Two-dimensional color saturation/brightness picker        |
| Color Field         | Text input for color values                               |
| Color Picker        | Full-featured color selection (area + sliders + swatches) |
| Color Slider        | Single-axis color channel slider                          |
| Color Swatch        | Visual color preview block                                |
| Color Swatch Picker | Grid of selectable color swatches                         |
| Color Wheel         | Circular hue selection                                    |
| Contextual Help     | Inline help text or popover for a field                   |
| File Upload         | Drag-and-drop file upload with validation                 |
| Image Cropper       | Interactive image crop and resize                         |
| QR Code             | QR code generator                                         |
| Signature Pad       | Freehand signature capture                                |
| Timer               | Countdown and stopwatch                                   |

### Utility

| Component         | Description                                                 |
| ----------------- | ----------------------------------------------------------- |
| Action Group      | Grouped set of related action buttons                       |
| Ars Provider      | Root provider for theme, locale, and platform configuration |
| As Child          | Render-prop pattern for custom element rendering            |
| Button            | Clickable action trigger                                    |
| Client Only       | Suppresses children during SSR                              |
| Dismissable       | Handles outside-click and escape-key dismissal              |
| Download Trigger  | Button that initiates a file download                       |
| Drop Zone         | Drag-and-drop target area                                   |
| Field             | Form field wrapper with label, description, and error       |
| Fieldset          | Groups related form fields with a legend                    |
| Focus Ring        | Visible focus indicator                                     |
| Focus Scope       | Traps or restores focus within a subtree                    |
| Form              | Form submission and validation container                    |
| Group             | Generic semantic grouping                                   |
| Heading           | Auto-leveled accessible heading                             |
| Highlight         | Text substring highlighting                                 |
| Keyboard          | Displays keyboard shortcut indicators                       |
| Landmark          | ARIA landmark region wrapper                                |
| Live Region       | ARIA live region for dynamic announcements                  |
| Separator         | Visual and semantic content divider                         |
| Swap              | Animated transition between two states                      |
| Toggle            | Pressable toggle with on/off state                          |
| Toggle Button     | Button with pressed/unpressed state                         |
| Toggle Group      | Mutually exclusive or multi-select toggle set               |
| Visually Hidden   | Content hidden visually but accessible to screen readers    |
| Z-Index Allocator | Manages z-index stacking across the application             |

## Architecture

```filetree
crates/
  ars-core/           State machine engine (no_std compatible)
  ars-a11y/           ARIA types, focus management, keyboard navigation, screen reader utilities
  ars-i18n/           Locale, RTL, formatting, calendars (ICU4X or browser Intl backend)
  ars-interactions/   Press, hover, focus, long press, move, drag-and-drop
  ars-collections/    Collection trait, selection models, virtualization, async loading
  ars-forms/          Validation, form context, field binding
  ars-dom/            web-sys DOM utilities, positioning, portals, scroll lock, URL sanitization
  ars-derive/         Proc macros: #[derive(HasId)], #[derive(ComponentPart)]
  ars-leptos/         Leptos adapter components
  ars-dioxus/         Dioxus adapter components
```

```diagram
                  ars-core (no_std)
                 /    |    \
          ars-a11y  ars-i18n  ars-interactions
             |  \      |           |
   ars-collections  ars-forms      |
             |          |          |
             |     ars-dom  <------+
             |        /       \
          ars-leptos        ars-dioxus
```

Each component follows the same pattern:

1. **State machine** (`ars-core`) defines states, transitions, and computed ARIA attributes
2. **Connect** produces typed attribute maps for each component part -- no framework dependency
3. **Adapter** (`ars-leptos` / `ars-dioxus`) subscribes to the machine and renders framework-specific markup
4. **Props** are validated at compile time through typed builder patterns

## License

Licensed under either of

- MIT license ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.
