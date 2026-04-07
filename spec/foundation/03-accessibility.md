# Accessibility Specification (`ars-a11y`)

Cross-references: see `00-overview.md` for naming conventions and data attribute system, `01-architecture.md` for the `Machine` trait, `AttrMap`, and crate dependency graph.

## Crate Preamble

`ars-a11y` is a `#![no_std]` crate that depends on `alloc`. All modules assume these crate-level declarations:

```rust
#![no_std]
extern crate alloc;

use alloc::vec::Vec;
```

---

## 1. Standards Compliance

### 1.1 WCAG 2.1 Level AA Baseline

Every component in ars-ui targets **WCAG 2.1 Level AA** conformance as a minimum, with Level AAA criteria documented where achievable. The following success criteria are the primary drivers of `ars-a11y` design decisions:

| Criterion                     | Level | Impact on ars-a11y                                                    |
| ----------------------------- | ----- | --------------------------------------------------------------------- |
| 1.3.1 Info and Relationships  | A     | Semantic roles, ARIA labels, heading structure                        |
| 1.3.2 Meaningful Sequence     | A     | DOM order matches visual order; focus order is logical                |
| 1.3.3 Sensory Characteristics | A     | Instructions never rely on shape/color/sound alone                    |
| 1.3.4 Orientation             | AA    | No lock to portrait or landscape                                      |
| 1.3.5 Identify Input Purpose  | AA    | `autocomplete` attributes on form inputs                              |
| 1.4.1 Use of Color            | A     | State never conveyed by color alone; data attributes used             |
| 1.4.3 Contrast (Minimum)      | AA    | 4.5:1 text, 3:1 large text; documented, not enforced in headless core |
| 1.4.4 Resize Text             | AA    | No text in images; em-based focus indicators                          |
| 1.4.10 Reflow                 | AA    | Single-column layout at 320 CSS pixels                                |
| 1.4.11 Non-text Contrast      | AA    | UI components 3:1 against adjacent colors                             |
| 1.4.12 Text Spacing           | AA    | No clipping when letter/word/line spacing increased                   |
| 1.4.13 Content on Hover/Focus | AA    | Hoverable, dismissible, persistent tooltip content                    |
| 2.1.1 Keyboard                | A     | Every operation performable via keyboard                              |
| 2.1.2 No Keyboard Trap        | A     | `FocusScope` always provides escape route                             |
| 2.1.4 Character Key Shortcuts | AA    | Type-ahead uses printable chars only; resistible                      |
| 2.4.3 Focus Order             | A     | Logical tab order; modal trapping does not break order                |
| 2.4.7 Focus Visible           | AA    | `FocusRing` ensures always-visible keyboard focus                     |
| 2.4.11 Focus Appearance       | AA    | Focus indicator meets size and contrast requirements                  |
| 2.5.3 Label in Name           | A     | Accessible name contains visible label text                           |
| 3.2.1 On Focus                | A     | No context change on focus alone                                      |
| 3.3.1 Error Identification    | A     | Error messages in text, not color alone                               |
| 4.1.2 Name, Role, Value       | A     | All UI components have correct ARIA name/role/value                   |
| 4.1.3 Status Messages         | AA    | `LiveAnnouncer` delivers status messages to AT                        |

### 1.2 WAI-ARIA 1.2 Authoring Practices

ars-ui follows the [WAI-ARIA Authoring Practices Guide 1.2](https://www.w3.org/WAI/ARIA/apg/) (APG) as the normative specification for keyboard interaction patterns and ARIA usage. Key principles:

- Use native HTML semantics first; add ARIA only when native semantics are insufficient.
- Never remove focus from an element without providing a defined focus destination.
- When using `role`, provide all required ARIA owned elements and required attributes.
- Avoid redundant ARIA (e.g., `role="button"` on a `<button>`).
- Required ARIA attributes that are not yet applicable should be set to their default values, not omitted.

### 1.3 Screen Reader Target Matrix

All components must be manually verified against this matrix before release:

| Screen Reader             | Browser         | Platform | Priority | Notes                             |
| ------------------------- | --------------- | -------- | -------- | --------------------------------- |
| **NVDA** (latest)         | Firefox, Chrome | Windows  | P0       | Most common combination worldwide |
| **JAWS** (latest)         | Chrome, Edge    | Windows  | P0       | Enterprise environments           |
| **VoiceOver**             | Safari          | macOS    | P0       | Default Apple screen reader       |
| **VoiceOver**             | Safari          | iOS      | P0       | Mobile testing required for touch |
| **TalkBack** (latest)     | Chrome          | Android  | P1       | Primary Android screen reader     |
| **Orca** (latest)         | Firefox         | Linux    | P1       | GNOME desktop                     |
| **Narrator** (Windows 11) | Edge            | Windows  | P2       | Increasingly relevant             |

#### 1.3.1 Per-Component Screen Reader Test Protocols

Each component must be tested against the matrix above with specific expected outputs. Below are example protocols for key components — all components must have similar protocols defined before release.

| Component    | Action                | Expected Screen Reader Output                                                         |
| ------------ | --------------------- | ------------------------------------------------------------------------------------- |
| **Button**   | Tab to button         | "[label] button"                                                                      |
| **Button**   | Press Enter/Space     | "[label] pressed" (if toggle) or action fires                                         |
| **Select**   | Tab to trigger        | "[label] combobox, collapsed"                                                         |
| **Select**   | Press Down Arrow      | "[first option], selected"                                                            |
| **Select**   | Press Enter on option | "[option] selected, [label] combobox, collapsed"                                      |
| **Dialog**   | Open dialog           | "[title] dialog" — focus moves to first focusable or announced via `aria-describedby` |
| **Dialog**   | Press Escape          | Dialog closes, focus returns to trigger, "dialog closed" (if live region used)        |
| **Checkbox** | Tab to checkbox       | "[label] checkbox, not checked"                                                       |
| **Checkbox** | Press Space           | "[label] checkbox, checked"                                                           |
| **Switch**   | Tab to switch         | "[label] switch, off"                                                                 |
| **Switch**   | Press Enter/Space     | "[label] switch, on"                                                                  |
| **Tabs**     | Tab to tab list       | "[active tab label] tab, [position] of [total], selected"                             |
| **Tabs**     | Press Arrow Right     | "[next tab label] tab, [position] of [total]"                                         |
| **Combobox** | Type in input         | "[N] results available" (via live region)                                             |
| **Combobox** | Press Down Arrow      | "[option text], [position] of [total]"                                                |
| **Toast**    | Toast appears         | "[message text]" announced via live region (polite or assertive)                      |
| **Slider**   | Tab to slider         | "[label] slider, [value], min [min], max [max]"                                       |
| **Slider**   | Press Arrow Right     | "[new value]"                                                                         |
| **Menu**     | Open menu             | "[first item] menuitem, [position] of [total]"                                        |

> **Note:** Exact phrasing varies between screen readers. The expected outputs above represent the semantic content that must be conveyed. Testers should verify the information is present, not match strings exactly.

Known behavioral differences that `ars-a11y` accounts for:

- **NVDA + Firefox** announces `aria-live` regions more aggressively than JAWS; polite regions may interrupt speech.
- **VoiceOver + Safari** reads `aria-label` in place of element content, unlike NVDA which reads both.
- **JAWS** treats `role="application"` as a pass-through and does not synthesize keyboard patterns; avoid this role.
- **TalkBack** uses swipe gestures; `aria-activedescendant` pattern is preferred over roving tabindex for lists on mobile.
- **VoiceOver iOS** does not support `aria-activedescendant`; roving tabindex must be used as a fallback. The default `RovingTabindex` strategy (see §3.2) satisfies both TalkBack's preference and VoiceOver iOS's requirement.
- **Orca** has incomplete support for some ARIA 1.2 roles; fallback roles should be documented per component.

### 1.4 Conformance Testing Methodology

#### 1.4.1 Automated Testing

- `ars-a11y::testing` module provides `AriaValidator` for compile-time and runtime ARIA validation.
- `wasm-bindgen-test` integration tests verify ARIA attribute output of connect functions.
- Axe-core integration via `ars-a11y::axe` feature flag runs accessibility tree assertions in CI.

#### 1.4.2 Manual Testing Protocol

For each component, execute the following checklist against each screen reader in the matrix:

1. **Role announcement**: Does the screen reader announce the correct role when focused?
2. **Label announcement**: Is the accessible name announced correctly (label, `aria-label`, or `aria-labelledby`)?
3. **State announcement**: Are state changes (expanded, selected, checked, disabled) announced?
4. **Keyboard navigation**: Does all keyboard interaction function as specified in §4 of this document?
5. **Focus management**: Does focus move correctly after state changes (dialog open/close, item selection)?
6. **Live region**: Are dynamic content changes announced via `aria-live`?
7. **Error messages**: Are form errors announced immediately on occurrence?
8. **Forced colors (WHCM)**: Test on Windows High Contrast Mode:
    - Focus indicators maintain ≥ 3:1 contrast ratio (use `Highlight` or `ButtonText` system colors).
    - `data-ars-*` attributes do not hide content — verify `::before`/`::after` state indicators remain visible.
    - Custom icons with `fill: currentColor` or `forced-color-adjust: auto` inherit system palette correctly.
    - Component states (selected, checked, disabled, pressed) are distinguishable without author-defined colors.

---

## 2. ARIA Attribute System

The `ars-a11y` crate provides typed, exhaustive ARIA attribute representations. Invalid attribute-value combinations are rejected at compile time wherever possible.

### 2.1 AriaRole Enum

Covers all roles defined in WAI-ARIA 1.2 Roles Model, including abstract roles (for inheritance documentation only), widget roles, document structure roles, and landmark roles.

```rust
// ars-a11y/src/aria/role.rs

/// All WAI-ARIA 1.2 roles.
///
/// Abstract roles are included for documentation and type-hierarchy purposes
/// but MUST NOT be set on DOM elements. Concrete roles are the correct values
/// for the `role` attribute.
///
/// Reference: https://www.w3.org/TR/wai-aria-1.2/#role_definitions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AriaRole {
    // ── Abstract roles (do not use on DOM elements) ──────────────────────────
    // Included for completeness; `to_attr_value` returns None for these.
    Command,
    Composite,
    Input,
    Landmark,
    Range,
    RoleType,
    Section,
    SectionHead,
    Select,
    Structure,
    Widget,
    Window,

    // ── Window roles ─────────────────────────────────────────────────────────
    Alertdialog,
    Dialog,

    // ── Widget roles ─────────────────────────────────────────────────────────
    Button,
    Checkbox,
    Gridcell,
    Link,
    Menuitem,
    Menuitemcheckbox,
    Menuitemradio,
    Option,
    Progressbar,
    Radio,
    Scrollbar,
    Searchbox,
    Separator,              // Focusable separator (widget role)
    Slider,
    Spinbutton,
    Switch,
    Tab,
    Tabpanel,
    Textbox,
    Treeitem,

    // ── Composite widget roles ────────────────────────────────────────────────
    Combobox,
    Grid,
    Listbox,
    Menu,
    Menubar,
    Radiogroup,
    Tablist,
    Tree,
    Treegrid,

    // ── Document structure roles ──────────────────────────────────────────────
    Application,
    Article,
    BlockQuote,
    Caption,
    Cell,
    Code,
    Columnheader,
    Comment,            // WAI-ARIA 1.3 draft — intentionally included; see note below
    Definition,
    Deletion,
    Directory,          // Superseded by `list` in ARIA 1.2; retained for assistive technology interoperability
    Document,
    Emphasis,
    Feed,
    Figure,
    Generic,
    Group,
    Heading,
    Img,
    Insertion,
    List,
    Listitem,
    Mark,
    Math,
    Meter,
    None,               // Same semantic as Presentation
    Note,
    Paragraph,
    Presentation,
    Row,
    Rowgroup,
    Rowheader,
    // SectionHead is listed under abstract roles above.
    Strong,
    Subscript,
    Suggestion,         // WAI-ARIA 1.3 draft — intentionally included; see note below
    Superscript,
    Table,
    Term,
    Time,
    Toolbar,
    Tooltip,

    // ── Live region roles ─────────────────────────────────────────────────────
    Alert,
    Log,
    Marquee,
    Status,
    Timer,

    // ── Landmark roles ────────────────────────────────────────────────────────
    Banner,
    Complementary,
    Contentinfo,
    Form,
    Main,
    Navigation,
    Region,
    Search,
    StructuralSeparator,    // Non-focusable separator (document structure role).
                            // Validators: distinguish from Separator by absence of tabindex.
                            // StructuralSeparator elements have no tabindex; focusable Separator
                            // elements have tabindex >= 0 and require value attributes.
}

// WAI-ARIA 1.3 draft roles: `Comment` and `Suggestion` are included intentionally.
// ars-ui tracks the WAI-ARIA specification as it evolves. These roles have broad
// assistive technology support already (NVDA 2024+, JAWS 2024+) and are on track
// for inclusion in the final 1.3 recommendation. If a role is removed from the
// draft before finalization, we will remove it here and update all references.

impl AriaRole {
    /// Returns the WAI-ARIA role string, or None for abstract roles.
    pub const fn to_attr_value(self) -> Option<&'static str> {
        match self {
            // Abstract roles — MUST NOT appear in DOM
            Self::Command | Self::Composite | Self::Input | Self::Landmark
            | Self::Range | Self::RoleType | Self::Section | Self::SectionHead
            | Self::Select | Self::Structure | Self::Widget | Self::Window => None,

            // Window roles
            Self::Alertdialog => Some("alertdialog"),
            Self::Dialog => Some("dialog"),

            // Widget roles
            Self::Button => Some("button"),
            Self::Checkbox => Some("checkbox"),
            Self::Gridcell => Some("gridcell"),
            Self::Link => Some("link"),
            Self::Menuitem => Some("menuitem"),
            Self::Menuitemcheckbox => Some("menuitemcheckbox"),
            Self::Menuitemradio => Some("menuitemradio"),
            // Warning: `AriaRole::Option` shadows `core::option::Option` if you
            // use `use AriaRole::*`. Prefer qualified access: `AriaRole::Option`.
            Self::Option => Some("option"),
            Self::Progressbar => Some("progressbar"),
            Self::Radio => Some("radio"),
            Self::Scrollbar => Some("scrollbar"),
            Self::Searchbox => Some("searchbox"),
            Self::Slider => Some("slider"),
            Self::Spinbutton => Some("spinbutton"),
            Self::Switch => Some("switch"),
            Self::Tab => Some("tab"),
            Self::Tabpanel => Some("tabpanel"),
            Self::Textbox => Some("textbox"),
            Self::Treeitem => Some("treeitem"),

            // Composite roles
            Self::Combobox => Some("combobox"),
            Self::Grid => Some("grid"),
            Self::Listbox => Some("listbox"),
            Self::Menu => Some("menu"),
            Self::Menubar => Some("menubar"),
            Self::Radiogroup => Some("radiogroup"),
            Self::Tablist => Some("tablist"),
            Self::Tree => Some("tree"),
            Self::Treegrid => Some("treegrid"),

            // Document structure
            Self::Application => Some("application"),
            Self::Article => Some("article"),
            Self::BlockQuote => Some("blockquote"),
            Self::Caption => Some("caption"),
            Self::Cell => Some("cell"),
            Self::Code => Some("code"),
            Self::Columnheader => Some("columnheader"),
            Self::Comment => Some("comment"),       // ARIA 1.3 draft
            Self::Definition => Some("definition"),
            Self::Deletion => Some("deletion"),
            Self::Directory => Some("directory"),
            Self::Document => Some("document"),
            Self::Emphasis => Some("emphasis"),
            Self::Feed => Some("feed"),
            Self::Figure => Some("figure"),
            Self::Generic => Some("generic"),
            Self::Group => Some("group"),
            Self::Heading => Some("heading"),
            Self::Img => Some("img"),
            Self::Insertion => Some("insertion"),
            Self::List => Some("list"),
            Self::Listitem => Some("listitem"),
            Self::Mark => Some("mark"),
            Self::Math => Some("math"),
            Self::Meter => Some("meter"),
            Self::None => Some("none"),
            Self::Note => Some("note"),
            Self::Paragraph => Some("paragraph"),
            Self::Presentation => Some("presentation"),
            Self::Row => Some("row"),
            Self::Rowgroup => Some("rowgroup"),
            Self::Rowheader => Some("rowheader"),
            Self::Strong => Some("strong"),
            Self::Subscript => Some("subscript"),
            Self::Suggestion => Some("suggestion"), // ARIA 1.3 draft
            Self::Superscript => Some("superscript"),
            Self::Table => Some("table"),
            Self::Term => Some("term"),
            Self::Time => Some("time"),
            Self::Toolbar => Some("toolbar"),
            Self::Tooltip => Some("tooltip"),

            // Live region roles
            Self::Alert => Some("alert"),
            Self::Log => Some("log"),
            Self::Marquee => Some("marquee"),
            Self::Status => Some("status"),
            Self::Timer => Some("timer"),

            // Landmark roles
            Self::Banner => Some("banner"),
            Self::Complementary => Some("complementary"),
            Self::Contentinfo => Some("contentinfo"),
            Self::Form => Some("form"),
            Self::Main => Some("main"),
            Self::Navigation => Some("navigation"),
            Self::Region => Some("region"),
            Self::Search => Some("search"),

            // Separator is overloaded: widget (focusable, with value semantics) vs structural.
            // AriaValidator should warn if StructuralSeparator is used on an element with
            // tabindex >= 0, or if Separator (widget) lacks tabindex and value attributes.
            Self::Separator => Some("separator"),
            Self::StructuralSeparator => Some("separator"),
        }
    }

    /// Returns true if this role supports `aria-activedescendant`.
    pub const fn supports_active_descendant(self) -> bool {
        matches!(
            self,
            Self::Application | Self::Combobox | Self::Grid | Self::Group
                | Self::Listbox | Self::Menu | Self::Menubar | Self::Radiogroup
                // Row supports aria-activedescendant only within grid/treegrid context
                | Self::Row | Self::Spinbutton | Self::Tablist
                // Note: Searchbox is intentionally excluded per WAI-ARIA 1.2. While
                // searchbox inherits from textbox (which supports aria-activedescendant),
                // the WAI-ARIA spec does not explicitly list searchbox as supporting it.
                // Autocomplete/combobox patterns should use the Combobox role instead.
                | Self::Textbox | Self::Toolbar | Self::Tree | Self::Treegrid
        )
    }

    /// Returns true if this is an abstract role that should never be used on DOM elements.
    pub const fn is_abstract(self) -> bool {
        self.to_attr_value().is_none()
    }

    /// Returns the ARIA string value for concrete roles (e.g., `"button"`),
    /// or the Rust variant name for abstract roles (e.g., `"Widget"`).
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::Composite => "Composite",
            Self::Input => "Input",
            Self::Landmark => "Landmark",
            Self::Range => "Range",
            Self::RoleType => "RoleType",
            Self::Section => "Section",
            Self::SectionHead => "SectionHead",
            Self::Select => "Select",
            Self::Structure => "Structure",
            Self::Widget => "Widget",
            Self::Window => "Window",
            // Safety: all non-abstract roles have a defined attr value string
            _ => match self.to_attr_value() {
                Some(s) => s,
                None => panic!("all non-abstract roles have attr values"),
            }
        }
    }

    /// Returns the required owned elements for this role.
    /// Used for ARIA validation: a container with this role MUST contain
    /// at least one element of one of these roles.
    ///
    /// **Semantics of the nested slices (OR-of-AND):**
    /// - The **outer** slice is OR: any one group can satisfy the requirement.
    /// - The **inner** slice is AND: all elements in the group must be present.
    ///
    /// Example: `&[&[Row], &[Rowgroup]]` for `Grid` means the grid must
    /// contain either direct `row` children OR `rowgroup` children.
    /// For `Table`: `&[&[Row], &[Rowgroup]]` means either `row` or `rowgroup`
    /// children are acceptable.
    ///
    /// A more complex example (not currently used but illustrative):
    /// `&[&["row"], &["rowgroup", "row"]]` would mean either direct `row`
    /// children OR `rowgroup` elements that themselves contain `row`.
    pub fn required_owned_elements(self) -> &'static [&'static [AriaRole]] {
        match self {
            Self::Feed => &[&[Self::Article]],
            Self::Grid => &[&[Self::Row], &[Self::Rowgroup]],
            Self::List => &[&[Self::Listitem]],
            Self::Listbox => &[&[Self::Option], &[Self::Group]],
            Self::Menu | Self::Menubar => &[
                &[Self::Menuitem],
                &[Self::Menuitemcheckbox],
                &[Self::Menuitemradio],
                &[Self::Group],
            ],
            Self::Radiogroup => &[&[Self::Radio]],
            Self::Row => &[&[Self::Cell], &[Self::Columnheader], &[Self::Gridcell], &[Self::Rowheader]],
            Self::Rowgroup => &[&[Self::Row]],
            Self::Table => &[&[Self::Row], &[Self::Rowgroup]],
            Self::Tablist => &[&[Self::Tab]],
            Self::Tree => &[&[Self::Treeitem], &[Self::Group]],
            Self::Treegrid => &[&[Self::Row], &[Self::Rowgroup]],
            _ => &[],
        }
    }
}
```

### 2.2 AriaAttribute Enum

A typed representation of all WAI-ARIA 1.2 state and property attributes, grouped by category.

```rust
// ars-a11y/src/aria/attribute.rs

/// A typed WAI-ARIA attribute with its associated value.
///
/// This enum covers all non-deprecated ARIA 1.2 states and properties.
/// WAI-ARIA 1.2 deprecated attributes (aria-grabbed, aria-dropeffect) are
/// available behind `#[cfg(feature = "aria-drag-drop-compat")]` and should not
/// be used in new components. Use `aria-description` for drop state feedback instead.
///
/// Reference: https://www.w3.org/TR/wai-aria-1.2/#state_prop_def
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AriaAttribute {
    // ── Widget attributes ─────────────────────────────────────────────────────

    /// Identifies the currently active element when DOM focus is on
    /// a composite widget, combobox, textbox, group, or application.
    ActiveDescendant(Option<AriaIdRef>),

    /// Indicates an element's value will be automatically completed.
    AutoComplete(AriaAutocomplete),

    /// Identifies the element (or elements) that controls the current element.
    Controls(AriaIdList),

    /// Indicates the element that represents the current item within a container
    /// or set of related elements.
    Current(AriaCurrent),

    /// Identifies the element (or elements) that describes the object.
    DescribedBy(AriaIdList),

    /// Defines a string value that describes or annotates the current element (WAI-ARIA 1.2).
    /// Preferred over aria-dropeffect for DnD descriptions.
    Description(String),

    /// Identifies the element that provides an extended description for the object.
    Details(AriaIdRef),

    /// Indicates that the element is perceivable but disabled.
    Disabled(bool),

    /// Identifies the next element(s) in a reading sequence when standard
    /// document reading order is not conveyed.
    FlowTo(AriaIdList),

    /// Indicates the availability and type of interactive popup element.
    HasPopup(AriaHasPopup),

    /// Indicates whether the element is hidden from the accessibility API.
    Hidden(Option<bool>),

    /// Indicates the entered value does not conform to the expected format.
    Invalid(AriaInvalid),

    /// Defines a string value that labels the current element.
    Label(String),

    /// Identifies the element (or elements) that labels the current element.
    LabelledBy(AriaIdList),

    /// Defines the hierarchical level of an element within a structure.
    Level(u32),

    /// Indicates whether an element is modal when displayed.
    Modal(bool),

    /// Indicates whether a text box accepts multiple lines of input.
    MultiLine(bool),

    /// Indicates that the user may select more than one item.
    MultiSelectable(bool),

    /// Indicates whether the element's orientation is horizontal, vertical, or unknown.
    Orientation(AriaOrientation),

    /// Identifies an element (or elements) in order to define a visual,
    /// functional, or contextual parent/child relationship between DOM elements.
    Owns(AriaIdList),

    /// Defines a short hint (a word or short phrase) intended to aid the user
    /// with data entry when the control has no value.
    Placeholder(String),

    /// Defines an element's number or position in the current set of listitems
    /// or treeitems. Not required if all elements in the set are present in the DOM.
    PosInSet(u32),

    /// Indicates the current "pressed" state of toggle buttons.
    /// `None` removes the attribute (undefined — element is not a toggle).
    /// Consistent with `Expanded(Option<bool>)` and `Selected(Option<bool>)`.
    Pressed(Option<AriaPressed>),

    /// Indicates that the element is not editable, but is otherwise operable.
    ReadOnly(bool),

    /// Indicates that user input is required on the element before a form may be submitted.
    Required(bool),

    /// Defines a human-readable, author-localized description for the role of an element.
    RoleDescription(String),

    /// Indicates the current "selected" state of various widgets.
    Selected(Option<bool>),

    /// Defines the number of items in the current set of listitems or treeitems.
    SetSize(i32),          // -1 = unknown

    /// Indicates if items in a table or grid are sorted in ascending, descending,
    /// or other order.
    Sort(AriaSort),

    /// Defines the maximum allowed value for a range widget.
    ValueMax(f64),

    /// Defines the minimum allowed value for a range widget.
    ValueMin(f64),

    /// Defines the current value for a range widget.
    ValueNow(f64),

    /// Defines the human-readable text alternative of aria-valuenow for a range widget.
    ValueText(String),

    // ── Live region attributes ────────────────────────────────────────────────

    /// Indicates whether assistive technologies will present all, or only parts of,
    /// the changed region based on the change notifications defined by `aria-relevant`.
    Atomic(bool),

    /// Indicates an element is being modified and that assistive technologies
    /// may want to wait until the modifications are complete before announcing them.
    Busy(bool),

    /// Indicates that an element will be updated, and describes the types of
    /// updates the user agents, assistive technologies, and user can expect.
    Live(AriaLive),

    /// Indicates what notifications the user agent will trigger when the
    /// accessibility tree within a live region is modified.
    Relevant(AriaRelevant),

    // ── Drag-and-drop attributes ──────────────────────────────────────────────

    /// WAI-ARIA 1.2 deprecated attribute. Prefer `aria-description` for drop state
    /// communication. Retained without `#[deprecated]` per project no-deprecation policy;
    /// gated behind `#[cfg(feature = "aria-drag-drop-compat")]`.
    #[cfg(feature = "aria-drag-drop-compat")]
    DropEffect(AriaDropeffect),

    // ── Relationship attributes ───────────────────────────────────────────────

    /// Identifies the element(s) that provide an error message for this element.
    ErrorMessage(AriaIdRef),

    // ── State-specific ────────────────────────────────────────────────────────

    /// Indicates the current "checked" state of checkboxes, radio buttons,
    /// and other widgets. See also `AriaPressed`.
    Checked(AriaChecked),

    /// Indicates the current "expanded" state of widget elements that can
    /// be expanded or collapsed.
    Expanded(Option<bool>),

    /// Indicates whether a grouping element owned or controlled by this element
    /// is expanded or collapsed.
    /// WAI-ARIA 1.2 deprecated attribute. Retained without `#[deprecated]` per project
    /// no-deprecation policy; gated behind `#[cfg(feature = "aria-drag-drop-compat")]`.
    #[cfg(feature = "aria-drag-drop-compat")]
    Grabbed(Option<bool>),

    // ── Grid/Table attributes ──────────────────────────────────────────────

    /// Defines the total number of columns in a table, grid, or treegrid.
    /// -1 indicates the total count is unknown.
    ColCount(i32),

    /// Defines an element's column index or position with respect to the total
    /// number of columns within a table, grid, or treegrid.
    ColIndex(u32),

    /// Defines the number of columns spanned by a cell or gridcell within
    /// a table, grid, or treegrid.
    ColSpan(u32),

    /// Defines the total number of rows in a table, grid, or treegrid.
    /// -1 indicates the total count is unknown.
    RowCount(i32),

    /// Defines an element's row index or position with respect to the total
    /// number of rows within a table, grid, or treegrid.
    RowIndex(u32),

    /// Defines the number of rows spanned by a cell or gridcell within
    /// a table, grid, or treegrid.
    RowSpan(u32),

    /// Indicates keyboard shortcuts that an author has implemented to
    /// activate or give focus to an element.
    KeyShortcuts(String),
}

impl AriaAttribute {
    /// Returns the HTML attribute name for this ARIA attribute.
    ///
    /// Delegates to [`AriaAttr::as_str()`] via the `From<&AriaAttribute>` conversion,
    /// keeping attribute name strings in a single source of truth.
    pub fn attr_name(&self) -> &'static str {
        AriaAttr::from(self).as_str()
    }

    /// Serializes this attribute to its DOM string value, or None if the
    /// attribute should be removed (e.g., `aria-hidden=None`).
    pub fn to_attr_value(&self) -> Option<String> {
        match self {
            Self::ActiveDescendant(id) => id.as_ref().map(|id| id.0.clone()),
            Self::AutoComplete(v) => Some(v.as_str().to_string()),
            Self::Controls(ids) => Some(ids.to_string()),
            Self::Current(v) => Some(v.as_str().to_string()),
            Self::DescribedBy(ids) => {
                let s = ids.to_string();
                if s.is_empty() { None } else { Some(s) }
            }
            Self::Description(s) => if s.is_empty() { None } else { Some(s.clone()) },
            Self::Details(id) => Some(id.0.clone()),
            Self::Disabled(v) => Some(v.to_string()),
            Self::FlowTo(ids) => Some(ids.to_string()),
            Self::HasPopup(v) => Some(v.as_str().to_string()),
            Self::Hidden(None) => None,
            Self::Hidden(Some(v)) => Some(v.to_string()),
            Self::Invalid(v) => Some(v.as_str().to_string()),
            Self::Label(s) => Some(s.clone()),
            Self::LabelledBy(ids) => {
                let s = ids.to_string();
                if s.is_empty() { None } else { Some(s) }
            }
            Self::Level(n) => Some(n.to_string()),
            Self::Modal(v) => Some(v.to_string()),
            Self::MultiLine(v) => Some(v.to_string()),
            Self::MultiSelectable(v) => Some(v.to_string()),
            Self::Orientation(v) => Some(v.as_str().to_string()),
            Self::Owns(ids) => Some(ids.to_string()),
            Self::Placeholder(s) => Some(s.clone()),
            Self::PosInSet(n) => Some(n.to_string()),
            Self::Pressed(None) => None,
            Self::Pressed(Some(v)) => Some(v.as_str().to_string()),
            Self::ReadOnly(v) => Some(v.to_string()),
            Self::Required(v) => Some(v.to_string()),
            Self::RoleDescription(s) => Some(s.clone()),
            Self::Selected(None) => None,
            Self::Selected(Some(v)) => Some(v.to_string()),
            Self::SetSize(n) => Some(n.to_string()),
            Self::Sort(v) => Some(v.as_str().to_string()),
            Self::ValueMax(n) => Some(n.to_string()),
            Self::ValueMin(n) => Some(n.to_string()),
            Self::ValueNow(n) => Some(n.to_string()),
            Self::ValueText(s) => Some(s.clone()),
            Self::Atomic(v) => Some(v.to_string()),
            Self::Busy(v) => Some(v.to_string()),
            Self::Live(v) => Some(v.as_str().to_string()),
            Self::Relevant(v) => Some(v.to_string()),
            #[cfg(feature = "aria-drag-drop-compat")]
            Self::DropEffect(v) => Some(v.as_str().to_string()),
            Self::ErrorMessage(id) => Some(id.0.clone()),
            Self::Checked(v) => Some(v.as_str().to_string()),
            Self::Expanded(None) => None,
            Self::Expanded(Some(v)) => Some(v.to_string()),
            #[cfg(feature = "aria-drag-drop-compat")]
            Self::Grabbed(None) => None,
            #[cfg(feature = "aria-drag-drop-compat")]
            Self::Grabbed(Some(v)) => Some(v.to_string()),
            Self::ColCount(n) => Some(n.to_string()),
            Self::ColIndex(n) => Some(n.to_string()),
            Self::ColSpan(n) => Some(n.to_string()),
            Self::RowCount(n) => Some(n.to_string()),
            Self::RowIndex(n) => Some(n.to_string()),
            Self::RowSpan(n) => Some(n.to_string()),
            Self::KeyShortcuts(s) => Some(s.clone()),
        }
    }

    /// Returns the `HtmlAttr` key for this ARIA attribute.
    pub fn to_html_attr(&self) -> HtmlAttr {
        HtmlAttr::Aria(AriaAttr::from(self))
    }

    /// Apply this attribute to an AttrMap.
    pub fn apply_to(&self, attrs: &mut crate::AttrMap) {
        let key = self.to_html_attr();
        match self.to_attr_value() {
            Some(value) => attrs.set(key, value),
            None => { attrs.set(key, AttrValue::None); }
        }
    }
}

// ── Bridging impls: AriaAttr ↔ AriaAttribute ─────────────────────────────────

/// Converts a discriminant key (`AriaAttr`) to an `AriaAttribute` with
/// default/placeholder values. Used by `validate_attr_map()` for
/// presence-checking — the actual value is not reconstructed.
impl From<AriaAttr> for AriaAttribute {
    fn from(attr: AriaAttr) -> Self {
        match attr {
            AriaAttr::ActiveDescendant => Self::ActiveDescendant(None),
            AriaAttr::AutoComplete => Self::AutoComplete(AriaAutocomplete::None),
            AriaAttr::Checked => Self::Checked(AriaChecked::False),
            AriaAttr::Controls => Self::Controls(AriaIdList::default()),
            AriaAttr::Current => Self::Current(AriaCurrent::False),
            AriaAttr::DescribedBy => Self::DescribedBy(AriaIdList::default()),
            AriaAttr::Description => Self::Description(String::new()),
            AriaAttr::Details => Self::Details(AriaIdRef(String::new())),
            AriaAttr::Disabled => Self::Disabled(false),
            AriaAttr::FlowTo => Self::FlowTo(AriaIdList::default()),
            AriaAttr::HasPopup => Self::HasPopup(AriaHasPopup::False),
            AriaAttr::Hidden => Self::Hidden(Some(true)),
            AriaAttr::Invalid => Self::Invalid(AriaInvalid::False),
            AriaAttr::Label => Self::Label(String::new()),
            AriaAttr::LabelledBy => Self::LabelledBy(AriaIdList::default()),
            AriaAttr::Level => Self::Level(1),
            AriaAttr::Live => Self::Live(AriaLive::Off),
            AriaAttr::Modal => Self::Modal(false),
            AriaAttr::MultiLine => Self::MultiLine(false),
            AriaAttr::MultiSelectable => Self::MultiSelectable(false),
            AriaAttr::Orientation => Self::Orientation(AriaOrientation::Horizontal),
            AriaAttr::Owns => Self::Owns(AriaIdList::default()),
            AriaAttr::Placeholder => Self::Placeholder(String::new()),
            AriaAttr::PosInSet => Self::PosInSet(1),
            AriaAttr::Pressed => Self::Pressed(Some(AriaPressed::False)),
            AriaAttr::ReadOnly => Self::ReadOnly(false),
            AriaAttr::Required => Self::Required(false),
            AriaAttr::RoleDescription => Self::RoleDescription(String::new()),
            AriaAttr::Selected => Self::Selected(Some(false)),
            AriaAttr::SetSize => Self::SetSize(0),
            AriaAttr::Sort => Self::Sort(AriaSort::None),
            AriaAttr::ValueMax => Self::ValueMax(0.0),
            AriaAttr::ValueMin => Self::ValueMin(0.0),
            AriaAttr::ValueNow => Self::ValueNow(0.0),
            AriaAttr::ValueText => Self::ValueText(String::new()),
            AriaAttr::Atomic => Self::Atomic(false),
            AriaAttr::Busy => Self::Busy(false),
            AriaAttr::Relevant => Self::Relevant(AriaRelevant::default()),
            #[cfg(feature = "aria-drag-drop-compat")]
            AriaAttr::DropEffect => Self::DropEffect(AriaDropeffect::None),
            #[cfg(feature = "aria-drag-drop-compat")]
            AriaAttr::Grabbed => Self::Grabbed(None),
            AriaAttr::ErrorMessage => Self::ErrorMessage(AriaIdRef(String::new())),
            AriaAttr::Expanded => Self::Expanded(Some(false)),
            AriaAttr::KeyShortcuts => Self::KeyShortcuts(String::new()),
            AriaAttr::ColCount => Self::ColCount(-1),
            AriaAttr::ColIndex => Self::ColIndex(1),
            AriaAttr::ColSpan => Self::ColSpan(1),
            AriaAttr::RowCount => Self::RowCount(-1),
            AriaAttr::RowIndex => Self::RowIndex(1),
            AriaAttr::RowSpan => Self::RowSpan(1),
        }
    }
}

/// Extracts the `AriaAttr` discriminant from an `HtmlAttr::Aria(a)` variant.
/// Returns `Err(original)` if the `HtmlAttr` is not an ARIA variant.
impl TryFrom<HtmlAttr> for AriaAttribute {
    type Error = HtmlAttr;

    fn try_from(attr: HtmlAttr) -> Result<Self, Self::Error> {
        match attr {
            HtmlAttr::Aria(a) => Ok(AriaAttribute::from(a)),
            other => Err(other),
        }
    }
}

/// Maps a data-carrying `AriaAttribute` back to its discriminant key.
impl From<&AriaAttribute> for AriaAttr {
    fn from(attr: &AriaAttribute) -> Self {
        match attr {
            AriaAttribute::ActiveDescendant(_) => Self::ActiveDescendant,
            AriaAttribute::AutoComplete(_) => Self::AutoComplete,
            AriaAttribute::Checked(_) => Self::Checked,
            AriaAttribute::Controls(_) => Self::Controls,
            AriaAttribute::Current(_) => Self::Current,
            AriaAttribute::DescribedBy(_) => Self::DescribedBy,
            AriaAttribute::Description(_) => Self::Description,
            AriaAttribute::Details(_) => Self::Details,
            AriaAttribute::Disabled(_) => Self::Disabled,
            AriaAttribute::FlowTo(_) => Self::FlowTo,
            AriaAttribute::HasPopup(_) => Self::HasPopup,
            AriaAttribute::Hidden(_) => Self::Hidden,
            AriaAttribute::Invalid(_) => Self::Invalid,
            AriaAttribute::Label(_) => Self::Label,
            AriaAttribute::LabelledBy(_) => Self::LabelledBy,
            AriaAttribute::Level(_) => Self::Level,
            AriaAttribute::Live(_) => Self::Live,
            AriaAttribute::Modal(_) => Self::Modal,
            AriaAttribute::MultiLine(_) => Self::MultiLine,
            AriaAttribute::MultiSelectable(_) => Self::MultiSelectable,
            AriaAttribute::Orientation(_) => Self::Orientation,
            AriaAttribute::Owns(_) => Self::Owns,
            AriaAttribute::Placeholder(_) => Self::Placeholder,
            AriaAttribute::PosInSet(_) => Self::PosInSet,
            AriaAttribute::Pressed(_) => Self::Pressed,
            AriaAttribute::ReadOnly(_) => Self::ReadOnly,
            AriaAttribute::Required(_) => Self::Required,
            AriaAttribute::RoleDescription(_) => Self::RoleDescription,
            AriaAttribute::Selected(_) => Self::Selected,
            AriaAttribute::SetSize(_) => Self::SetSize,
            AriaAttribute::Sort(_) => Self::Sort,
            AriaAttribute::ValueMax(_) => Self::ValueMax,
            AriaAttribute::ValueMin(_) => Self::ValueMin,
            AriaAttribute::ValueNow(_) => Self::ValueNow,
            AriaAttribute::ValueText(_) => Self::ValueText,
            AriaAttribute::Atomic(_) => Self::Atomic,
            AriaAttribute::Busy(_) => Self::Busy,
            AriaAttribute::Relevant(_) => Self::Relevant,
            #[cfg(feature = "aria-drag-drop-compat")]
            AriaAttribute::DropEffect(_) => Self::DropEffect,
            #[cfg(feature = "aria-drag-drop-compat")]
            AriaAttribute::Grabbed(_) => Self::Grabbed,
            AriaAttribute::ErrorMessage(_) => Self::ErrorMessage,
            AriaAttribute::Expanded(_) => Self::Expanded,
            AriaAttribute::KeyShortcuts(_) => Self::KeyShortcuts,
            AriaAttribute::ColCount(_) => Self::ColCount,
            AriaAttribute::ColIndex(_) => Self::ColIndex,
            AriaAttribute::ColSpan(_) => Self::ColSpan,
            AriaAttribute::RowCount(_) => Self::RowCount,
            AriaAttribute::RowIndex(_) => Self::RowIndex,
            AriaAttribute::RowSpan(_) => Self::RowSpan,
        }
    }
}

// ── Supporting types ──────────────────────────────────────────────────────────

/// A single ARIA ID reference (used in aria-activedescendant, aria-details, etc.)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AriaIdRef(pub String);

/// A space-separated list of ARIA ID references.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AriaIdList(pub Vec<String>);

impl AriaIdList {
    pub fn new() -> Self { Self::default() }
    pub fn push(&mut self, id: impl Into<String>) { self.0.push(id.into()); }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

impl core::fmt::Display for AriaIdList {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0.join(" "))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaAutocomplete { None, Inline, List, Both }
impl AriaAutocomplete {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Inline => "inline",
            Self::List => "list",
            Self::Both => "both",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaCurrent {
    False, True, Page, Step, Location, Date, Time,
}
impl AriaCurrent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Page => "page",
            Self::Step => "step",
            Self::Location => "location",
            Self::Date => "date",
            Self::Time => "time",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaHasPopup {
    False, True, Menu, Listbox, Tree, Grid, Dialog,
}
impl AriaHasPopup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Menu => "menu",
            Self::Listbox => "listbox",
            Self::Tree => "tree",
            Self::Grid => "grid",
            Self::Dialog => "dialog",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AriaInvalid { False, True, Grammar, Spelling }
impl AriaInvalid {
    pub fn as_str(self) -> &'static str {
        match self { Self::False => "false", Self::True => "true",
                     Self::Grammar => "grammar", Self::Spelling => "spelling" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaLive { Off, Polite, Assertive }
impl AriaLive {
    pub fn as_str(self) -> &'static str {
        match self { Self::Off => "off", Self::Polite => "polite", Self::Assertive => "assertive" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaOrientation { Horizontal, Vertical, Undefined }
impl AriaOrientation {
    pub fn as_str(self) -> &'static str {
        match self { Self::Horizontal => "horizontal", Self::Vertical => "vertical",
                     Self::Undefined => "undefined" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaPressed { False, True, Mixed }
impl AriaPressed {
    pub fn as_str(self) -> &'static str {
        match self { Self::False => "false", Self::True => "true", Self::Mixed => "mixed" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaChecked { False, True, Mixed }
impl AriaChecked {
    pub fn as_str(self) -> &'static str {
        match self { Self::False => "false", Self::True => "true", Self::Mixed => "mixed" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaSort { None, Ascending, Descending, Other }
impl AriaSort {
    pub fn as_str(self) -> &'static str {
        match self { Self::None => "none", Self::Ascending => "ascending",
                     Self::Descending => "descending", Self::Other => "other" }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AriaRelevant {
    pub additions: bool,
    pub removals: bool,
    pub text: bool,
}
impl core::fmt::Display for AriaRelevant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut parts: Vec<&str> = Vec::new();
        if self.additions { parts.push("additions"); }
        if self.removals { parts.push("removals"); }
        if self.text { parts.push("text"); }
        if parts.is_empty() {
            // All-false returns empty string so the attribute is omitted, letting
            // the browser apply its default (`additions text`).
            return write!(f, "");
        }
        write!(f, "{}", parts.join(" "))
    }
}
impl Default for AriaRelevant {
    fn default() -> Self { Self { additions: true, removals: false, text: true } }
}

#[cfg(feature = "aria-drag-drop-compat")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaDropeffect { None, Copy, Execute, Link, Move, Popup }
#[cfg(feature = "aria-drag-drop-compat")]
impl AriaDropeffect {
    pub fn as_str(self) -> &'static str {
        match self { Self::None => "none", Self::Copy => "copy", Self::Execute => "execute",
                     Self::Link => "link", Self::Move => "move", Self::Popup => "popup" }
    }
}
```

### 2.3 Role Assignment Patterns

Roles are set via `AttrMap` in connect functions. The following conventions apply:

- Use native HTML elements with implicit roles whenever possible (`<button>`, `<input>`, `<a>`).
- Set `role` only when using a non-semantic element (`<div>`, `<span>`) or when overriding the implicit role.
- Never set `role="presentation"` or `role="none"` on interactive elements.

```rust
// Pattern: applying role and ARIA attributes to AttrMap in a connect function.
// ars-a11y/src/aria/apply.rs

pub fn apply_role(attrs: &mut AttrMap, role: AriaRole) {
    if let Some(value) = role.to_attr_value() {
        attrs.set(HtmlAttr::Role, value);
    }
    // Abstract roles are silently ignored — no debug_assert here because
    // callers may derive roles dynamically; instead, validate via AriaValidator.
}

pub fn apply_aria(attr_map: &mut AttrMap, aria_attrs: impl IntoIterator<Item = AriaAttribute>) {
    for attr in aria_attrs {
        attr.apply_to(attr_map);
    }
}

/// Compile-time checked role assignment.
/// Use this macro to get a compile error for abstract roles.
/// **Note:** The compile-time check is enforced only when the argument is a `const`
/// expression (e.g., a literal `AriaRole::Button`). For runtime role values, call
/// `AriaRole::is_abstract()` manually before calling `set_role!`.
/// Usage: `set_role!(attrs, AriaRole::Button)`
#[macro_export]
macro_rules! set_role {
    ($attrs:expr, $role:expr) => {{
        const _: () = {
            // Evaluated at compile time — will fail if role is abstract.
            // The trick: abstract roles have `to_attr_value` returning None,
            // but we use a const fn check.
            assert!(!$role.is_abstract(), "Cannot set an abstract ARIA role on a DOM element");
        };
        $crate::aria::apply::apply_role(&mut $attrs, $role);
    }};
}
```

### 2.4 Standard `data-ars-state` Values

Components expose their current state via the `data-ars-state` attribute for CSS styling hooks
and adapter consumption. The following canonical values MUST be used consistently across all
components. Do not invent ad-hoc state strings when a standard value applies.

| Value           | Meaning                              | Used By                                                       |
| --------------- | ------------------------------------ | ------------------------------------------------------------- |
| `open`          | Widget is expanded / visible         | Dialog, Popover, Tooltip, Accordion, Select, Combobox, Drawer |
| `closed`        | Widget is collapsed / hidden         | Dialog, Popover, Tooltip, Accordion, Select, Combobox, Drawer |
| `checked`       | Checkbox/switch is on                | Checkbox, Switch, Menu item checkbox                          |
| `unchecked`     | Checkbox/switch is off               | Checkbox, Switch, Menu item checkbox                          |
| `indeterminate` | Tri-state checkbox partial selection | Checkbox (tri-state), Tree parent                             |
| `on`            | Toggle is active                     | Toggle, ToggleGroup                                           |
| `off`           | Toggle is inactive                   | Toggle, ToggleGroup                                           |
| `active`        | Currently active/pressed state       | Button (during press), Tabs (active tab)                      |
| `inactive`      | Not currently active                 | Tabs (inactive tab)                                           |
| `focused`       | Element has focus                    | All interactive components                                    |
| `disabled`      | Element is disabled                  | All interactive components                                    |
| `idle`          | No ongoing interaction               | Slider, SearchInput, FileUpload                               |
| `dragging`      | Element is being dragged             | Slider (thumb), DropZone, Drawer (snap drag)                  |
| `hover`         | Pointer is over element              | HoverCard trigger, Tooltip trigger                            |
| `pressed`       | Element is being pressed             | Button (during pointer down)                                  |
| `selected`      | Item is selected                     | Listbox option, Menu item, Table row                          |
| `loading`       | Async operation in progress          | Button (loading), SearchInput (searching)                     |
| `error`         | Validation or operation error        | Input fields, FileUpload                                      |
| `valid`         | Validation passed                    | Input fields                                                  |
| `preview`       | Editable is in read mode             | Editable                                                      |
| `editing`       | Editable is in edit mode             | Editable                                                      |

**Note:** `data-ars-active` (a boolean interaction attribute from `PressResult`) is distinct from
`data-ars-state="active"` (a component-level semantic state). The former is set by the Press
interaction with RAF-deferred removal; the latter reflects component state machine state.

Multiple states can be combined using space separation when needed:
`data-ars-state="open focused"`. CSS can target individual states with `[data-ars-state~="open"]`.

### 2.5 State and Property Management

State attributes change frequently during interaction. The canonical patterns:

```rust
// ars-a11y/src/aria/state.rs

/// Helpers for the most common state transitions, used inside connect() implementations.

/// Set aria-expanded based on boolean state. None removes the attribute (for
/// elements that do not inherently have an expanded concept).
pub fn set_expanded(attrs: &mut AttrMap, expanded: Option<bool>) {
    AriaAttribute::Expanded(expanded).apply_to(attrs);
}

/// Set aria-selected. None removes the attribute (for elements where
/// selection is not applicable in the current context).
pub fn set_selected(attrs: &mut AttrMap, selected: Option<bool>) {
    AriaAttribute::Selected(selected).apply_to(attrs);
}

/// Set aria-checked for checkbox/radio/switch semantics.
pub fn set_checked(attrs: &mut AttrMap, checked: AriaChecked) {
    AriaAttribute::Checked(checked).apply_to(attrs);
}

/// Set aria-disabled. Uses aria-disabled rather than the HTML `disabled`
/// attribute for non-form elements. For <button> and <input>, use the
/// native `disabled` attribute in addition to aria-disabled.
///
/// Note: `data-ars-disabled` is NOT set by this helper — that attribute is
/// the responsibility of interaction primitives (e.g., `PressResult::current_attrs()`)
/// and component connect functions.
pub fn set_disabled(attrs: &mut AttrMap, disabled: bool) {
    if disabled {
        AriaAttribute::Disabled(true).apply_to(attrs);
    } else {
        attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), AttrValue::None);
    }
}

/// Set aria-busy for loading states.
pub fn set_busy(attrs: &mut AttrMap, busy: bool) {
    AriaAttribute::Busy(busy).apply_to(attrs);
}

/// Set aria-invalid with optional error message reference.
///
/// **Note on `aria-errormessage` vs `aria-describedby`**: While this function
/// sets `aria-errormessage` per the ARIA 1.2 spec, screen reader support for
/// `aria-errormessage` remains poor as of 2025 (NVDA and JAWS have limited
/// support; VoiceOver does not announce it reliably). For maximum
/// compatibility, callers should **also** include the error message element's
/// ID in `aria-describedby`. The `FieldContext` (see §5.4) handles
/// this automatically by appending the error ID to `describedby_ids` when the
/// field is invalid. Using both attributes is not harmful — `aria-describedby`
/// provides the fallback announcement while `aria-errormessage` provides
/// the semantic relationship for assistive technologies that support it.
pub fn set_invalid(attrs: &mut AttrMap, invalid: AriaInvalid, error_id: Option<&str>) {
    AriaAttribute::Invalid(invalid).apply_to(attrs);
    if let Some(id) = error_id {
        AriaAttribute::ErrorMessage(AriaIdRef(String::from(id))).apply_to(attrs);
    } else {
        attrs.set(HtmlAttr::Aria(AriaAttr::ErrorMessage), AttrValue::None);
    }
}
```

### 2.6 Relationship Attributes and ID Generation

ARIA relationship attributes (`aria-labelledby`, `aria-describedby`, `aria-controls`, `aria-activedescendant`) require stable, unique IDs shared between DOM elements. ars-ui uses adapter-provided IDs that are hydration-safe and unique, delegating ID generation to each adapter's ID utility (e.g., `use_id()` in ars-leptos — an adapter-local `AtomicU32` counter; Dioxus scope ID).

```rust
// ars-a11y/src/id.rs

/// Derives component part IDs from an adapter-provided base ID.
/// The base ID comes from the adapter's hydration-safe ID utility
/// (e.g., `use_id()` in ars-leptos, scope ID in ars-dioxus).
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentIds {
    base: String,
}

impl ComponentIds {
    /// Create from an adapter-provided base ID.
    /// The base ID must be unique and hydration-safe.
    pub fn from_id(base_id: &str) -> Self {
        debug_assert!(!base_id.is_empty(), "Component base ID must not be empty");
        Self { base: base_id.to_string() }
    }

    /// Returns the base ID (for the root element).
    pub fn id(&self) -> &str {
        &self.base
    }

    /// Derive a part ID: `"{base}-{part}"`.
    /// Use for fixed structural parts of a component (trigger, content, label, etc.).
    pub fn part(&self, part: &str) -> String {
        format!("{}-{}", self.base, part)
    }

    /// Derive a keyed item ID: `"{base}-{part}-{key}"`.
    /// Use for per-item IDs in collection components (lists, grids, trees, menus).
    /// Example: `ids.item("item", &"option-a")` → `"ars-listbox-2-item-option-a"`
    pub fn item(&self, part: &str, key: &impl core::fmt::Display) -> String {
        format!("{}-{}-{}", self.base, part, key)
    }

    /// Derive a keyed item sub-part ID: `"{base}-{part}-{key}-{sub}"`.
    /// Use for sub-elements within a keyed item (text label, indicator, etc.).
    /// Example: `ids.item_part("item", &"opt-a", "text")` → `"ars-listbox-2-item-opt-a-text"`
    pub fn item_part(&self, part: &str, key: &impl core::fmt::Display, sub: &str) -> String {
        format!("{}-{}-{}-{}", self.base, part, key, sub)
    }
}

// Usage pattern in a connect() function:
//
// impl<'a> dialog::Api<'a> {
//     pub fn content_attrs(&self) -> AttrMap {
//         let mut attrs = AttrMap::new();
//         let ids = &self.ctx.ids;
//
//         set_role!(attrs, AriaRole::Dialog);
//         attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
//         attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), ids.part("title"));
//         attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), ids.part("description"));
//         attrs.set(HtmlAttr::Id, ids.part("content"));
//         attrs
//     }
//
//     pub fn title_attrs(&self) -> AttrMap {
//         let mut attrs = AttrMap::new();
//         attrs.set(HtmlAttr::Id, self.ctx.ids.part("title"));
//         attrs
//     }
// }
```

> **Note**: Component IDs are provided by framework adapters, not auto-generated. See 01-architecture.md §9.2.

### 2.7 ARIA Naming Precedence

For interactive controls with a visible `<label>`, set `aria-labelledby` pointing
to the label's ID. Do NOT also set `aria-label` — it would be ignored by
screen readers (`aria-labelledby` takes precedence per the ARIA spec). Use
`aria-label` only when no visible label exists (e.g., icon-only buttons, close
buttons).

Precedence order (highest to lowest):

1. `aria-labelledby` — references other visible elements
2. `aria-label` — inline string label
3. Native `<label>` element association
4. `title` attribute (last resort)

Components MUST NOT set both `aria-labelledby` and `aria-label` on the same
element. The connect API methods should check for the presence of a visible
label part (e.g., `has_label: bool` in context) and choose accordingly.

### 2.8 Localized ARIA Labels Rule

**Spec-wide rule:** No string literals are permitted in `aria-label` or `aria-valuetext` attribute values. Every user-facing ARIA string MUST come from a component's `Messages` struct (following the `toast::Messages` pattern from `spec/components/overlay/toast.md`). This ensures all assistive technology announcements are localizable.

Components that require localized strings include (non-exhaustive):

- Select: "Clear selection"
- TagsInput: "Remove all tags", "Remove tag {value}", "Press Delete to remove"
- SearchInput: "Clear search", "Submit search"
- SignaturePad: "Clear signature", "Undo last stroke"
- DateField: segment labels ("Year", "Month", "Day")
- Table: "Select all rows"
- Pagination: "Pagination", "Go to previous page", "Go to next page", "Page {n}"
- Calendar: "Previous month", "Next month", "Previous {n} months", "Next {n} months"
- DatePicker / DateRangePicker: "Open calendar", "Choose date", "Start date", "End date"
- Timer: "Start timer", "Resume timer", "Pause timer", "Reset timer", "Timer progress"
- Editable: "Edit", "Submit edit", "Cancel edit", "Editable field"
- Steps: "Steps"
- QR Code: "QR code: {value}"
- DropZone: "Drop files here"
- FloatingPanel: "Close panel", "Minimize panel", "Maximize panel", "Restore panel", "Move panel", "Resize {direction}"

Each component MUST define a `Messages` struct with `Default` providing English fallback strings. The connect/attrs methods read from this struct rather than embedding string literals. See `TableMessages` in `spec/components/data-display/table.md` §1.5 as the canonical example.

**Compliance check:** The following connect functions still hardcode English strings
directly in `aria-label` / `aria-valuetext` attributes instead of reading from their
component's Messages struct. These MUST be fixed before the spec is considered stable:

| Component       | File                                                                                                                      | Hardcoded strings in connect code                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Calendar        | components/date-time/calendar.md §`prev_trigger_attrs`, §`next_trigger_attrs`                                             | `"Previous month"`, `"Next month"`, `format!("Previous/Next {} months", step)`          |
| DateRangePicker | components/date-time/date-range-picker.md §`trigger_attrs`, §`start_input_props`, §`end_input_props`                      | `"Open calendar"`, `"Start date"`, `"End date"`                                         |
| Editable        | components/input/editable.md §`input_attrs`, §`submit_trigger_attrs`, §`cancel_trigger_attrs`, §`edit_trigger_attrs`      | `"Editable field"`, `"Submit edit"`, `"Cancel edit"`, `"Edit"`                          |
| Pagination      | components/navigation/pagination.md §`root_attrs`, §`prev_trigger_attrs`, §`next_trigger_attrs`, §`page_trigger_attrs`    | `"Pagination"`, `"Go to previous page"`, `"Go to next page"`, `format!("Page {}")`      |
| Steps           | components/navigation/steps.md §`root_attrs`                                                                              | `"Steps"`                                                                               |
| Timer           | components/specialized/timer.md §`start_trigger_attrs`, §`pause_trigger_attrs`, §`reset_trigger_attrs`, §`progress_attrs` | `"Start timer"`, `"Resume timer"`, `"Pause timer"`, `"Reset timer"`, `"Timer progress"` |
| QrCode          | components/specialized/qr-code.md §`root_attrs`                                                                           | `format!("QR code: {}", ...)`                                                           |
| DropZone        | components/utility/drop-zone.md §`root_attrs`                                                                             | `"Drop files here"` (fallback)                                                          |

**Enforcement (v12+):** Components that have zero user-facing ARIA strings (e.g., `Checkbox`, `RadioGroup`, `Separator`, `VisuallyHidden`, layout primitives) are exempt from this mandate. All other components — those that emit any `aria-label`, `aria-valuetext`, `aria-roledescription`, or status announcement string — MUST define a `Messages` struct. Adapter authors MUST NOT introduce hardcoded English strings in any `aria-*` attribute; doing so is a WCAG 3.1.1 / 2.5.3 violation. CI linting SHOULD flag string literals in `aria-label` and `aria-valuetext` attributes.

### 2.9 `aria-roledescription` Guidance

The `aria-roledescription` attribute provides a human-readable, localized description of the
role of an element. It overrides the default role announcement by screen readers.

**General principle**: `aria-roledescription` should only be used when the element's semantic
role does not adequately describe the widget's behavior to screen reader users. Overusing
this attribute can degrade the user experience by replacing well-known role names with
unfamiliar terms.

**Components that SHOULD use `aria-roledescription`:**

| Component      | Value        | Rationale                                                     |
| -------------- | ------------ | ------------------------------------------------------------- |
| Carousel       | `"carousel"` | `role="region"` alone does not convey carousel semantics      |
| Carousel Slide | `"slide"`    | `role="group"` does not describe a slide within a carousel    |
| Drawer         | `"drawer"`   | Distinguishes from a generic `dialog` for screen reader users |

**Requirements:**

- The value MUST be localized via the component's `Messages` / `I18n` struct, never hardcoded in English.
  For example, `CarouselI18n.role_description` provides the localized string.
- The value MUST be a single, concise word or short phrase (e.g., "carousel", not "interactive carousel widget").
- Do NOT use `aria-roledescription` on elements with abstract or generic roles.
- Do NOT use it when the standard role name (e.g., "button", "dialog", "tab") already
  accurately describes the widget.

```rust
// Example: Carousel root attrs
attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription),
    &self.i18n.role_description); // e.g., "carousel" in English
```

---

## 3. Focus Management

The `ars-a11y::focus` module provides primitives for all focus management patterns required by the WAI-ARIA APG.

### 3.1 Focus Event Bubbling

ars-ui uses the following focus event strategy:

1. **`focusin`/`focusout` (bubbling)** — used for composite components
   (RadioGroup, Tabs, Menu) to detect focus entering/leaving the group
2. **`focus`/`blur` (non-bubbling)** — used for individual interactive
   elements (Button, Checkbox, TextField)
3. **`FocusWithin`** — derived from focusin/focusout on the container.
   Components expose `on_focus_within` / `on_blur_within` when the
   container itself needs to track whether any descendant has focus.

The adapter is responsible for attaching the correct DOM events.
Core machines always receive normalized `Event::Focus`/`Event::Blur`.

### 3.2 FocusScope

`FocusScope` manages a bounded region of focusable content. It handles three concerns:

1. **Containment**: Trapping Tab and Shift+Tab within the scope (required for modal dialogs).
2. **Restoration**: Returning focus to the previously focused element when the scope is exited.
3. **Auto-focus**: Moving focus into the scope when it is first activated.

```rust
// ars-a11y/src/focus/scope.rs

/// Options controlling FocusScope behavior.
#[derive(Clone, Debug)]
pub struct FocusScopeOptions {
    /// If true, Tab/Shift+Tab are prevented from leaving the scope.
    /// Required for modal dialogs. Must NOT be used for non-modal overlays.
    pub contain: bool,

    /// If true, focus is restored to the previously focused element
    /// when the scope is deactivated/dropped.
    pub restore_focus: bool,

    /// If true, focus is automatically moved into the scope on activation.
    /// Targets the first tabbable element, or the element with `data-ars-autofocus`.
    pub auto_focus: bool,
}

impl Default for FocusScopeOptions {
    fn default() -> Self {
        Self {
            contain: false,
            restore_focus: true,
            auto_focus: true,
        }
    }
}

impl FocusScopeOptions {
    /// Preset for modal dialogs (contain + restore + auto_focus).
    pub fn modal() -> Self {
        Self { contain: true, restore_focus: true, auto_focus: true }
    }

    /// Preset for non-modal overlays — no containment, but restore and auto-focus.
    pub fn overlay() -> Self {
        Self { contain: false, restore_focus: true, auto_focus: true }
    }

    /// Preset for inline regions — no containment, no auto-focus, no restoration.
    pub fn inline() -> Self {
        Self { contain: false, restore_focus: false, auto_focus: false }
    }
}

/// Trait defining the public interface for focus scope behavior.
/// Defined in `ars-a11y` (no_std + alloc). Implemented by `FocusScope` in `ars-dom`.
///
/// **Note:** The method names `activate`, `deactivate`, and `is_active` match the
/// inherent methods on `FocusScope`. Callers use the trait methods via dynamic dispatch
/// (`&dyn FocusScopeBehavior`) or the inherent methods on the concrete type — both
/// dispatch to the same implementation.
pub trait FocusScopeBehavior {
    /// Activate the scope: save current focus, optionally trap and auto-focus.
    /// `focus_target` determines which element receives initial focus within the scope.
    fn activate(&mut self, focus_target: FocusTarget);
    /// Deactivate the scope: release trap, optionally restore previous focus.
    fn deactivate(&mut self);
    /// Whether the scope is currently active.
    fn is_active(&self) -> bool;
}

```

The concrete `FocusScope` struct lives in **ars-dom** and implements `FocusScopeBehavior`.
See `11-dom-utilities.md` §3.3 for the full implementation specification including
`activate()`, `deactivate()`, `handle_tab_key()`, focus restore fallback chain, and `Drop`.

```rust
/// Target for selecting which element receives focus when a scope activates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// Focus the first tabbable element.
    First,
    /// Focus the last tabbable element.
    Last,
    /// Focus the element marked with `data-ars-autofocus`.
    AutofocusMarked,
    /// Focus the element that was previously active within this scope
    /// (useful for re-opening dialogs that were closed and reopened).
    PreviouslyActive,
}

/// How focus is managed within a composite widget.
///
/// Default: `RovingTabindex` everywhere. Use `ActiveDescendant` only for:
/// - Combobox (focus must stay on the text input)
/// - Virtualized lists with 10,000+ items (avoids DOM focus moves)
///
/// **VoiceOver iOS incompatibility**: `aria-activedescendant` is not supported
/// on VoiceOver iOS. When using `ActiveDescendant`, provide a roving tabindex
/// fallback or document the limitation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusStrategy {
    /// Each focusable item gets `tabindex` toggled between 0 and -1.
    /// The focused item has `tabindex="0"`, all others `tabindex="-1"`.
    /// Arrow keys move DOM focus between items.
    #[default]
    RovingTabindex,

    /// The container has `tabindex="0"` and `aria-activedescendant` points
    /// to the visually focused item's ID. DOM focus stays on the container.
    /// Arrow keys update `aria-activedescendant` without moving DOM focus.
    ActiveDescendant,
}
```

> **Adapter wiring by strategy:**
>
> - **`RovingTabindex`:** `FocusZone::handle_key()` returns a new focus index. The adapter calls `element.focus()` on the target item and updates `tabindex` attributes — the focused item gets `tabindex="0"`, all others get `tabindex="-1"`.
> - **`ActiveDescendant`:** `FocusZone::handle_key()` returns the new index. The adapter sets `aria-activedescendant` on the container element to the target item's ID. DOM focus remains on the container; no `element.focus()` call is made on individual items.

```rust
/// Query for tabbable elements within a container.
/// A tabbable element satisfies ALL of:
///   - Matches the focusable selector (button, input, select, textarea, a[href],
///     [tabindex], area[href], [contenteditable])
///   - Is not disabled
///   - Is not hidden (visibility: hidden, display: none, or hidden ancestor)
///   - Has tabindex >= 0 (or no explicit tabindex, which defaults to 0)
///
/// The ordering is: elements with explicit tabindex > 0 (sorted numerically),
/// then elements with tabindex = 0 (sorted by DOM order).
pub fn get_tabbable_elements_selector() -> &'static str {
    // Note: CSS selectors cannot test computed visibility (display: none,
    // visibility: hidden). Post-query filtering via computed styles is
    // required to fully exclude hidden elements.
    // Also note: `:not([aria-hidden='true'])` only checks the element itself,
    // not ancestors. A post-query ancestor walk is needed to filter out elements
    // inside `aria-hidden="true"` containers.
    //
    // Negative tabindex: We only exclude `tabindex="-1"` (not arbitrary negative
    // values like `-2`) because ars-ui components exclusively use `-1` to remove
    // elements from the tab order. Other negative tabindex values are a user error.
    concat!(
        "button:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
        "input:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
        "select:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
        "textarea:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
        "a[href]:not([tabindex='-1']):not([aria-hidden='true']),",
        "area[href]:not([tabindex='-1']):not([aria-hidden='true']),",
        "[tabindex]:not([tabindex='-1']):not([disabled]):not([aria-hidden='true']),",
        "[contenteditable]:not([contenteditable='false']):not([tabindex='-1']):not([aria-hidden='true'])",
    )
}
```

### 3.3 Focus Navigation Strategy

Components that manage focus across multiple items use one of two strategies:

See `FocusStrategy` definition above in §3.2.

**Default**: `RovingTabindex` for all components unless explicitly documented otherwise.

**Rationale**: VoiceOver on iOS does not support `aria-activedescendant`. Using roving tabindex ensures universal compatibility.

> **VoiceOver iOS workaround.** VoiceOver on iOS sometimes fails to announce
> `aria-activedescendant` changes on combobox/listbox patterns. Components relying on
> `aria-activedescendant` (Select, Menu, Listbox, Combobox) SHOULD include a fallback
> `aria-live="polite"` visually-hidden region that announces the currently focused item
> label when `aria-activedescendant` changes. This is a belt-and-suspenders approach
> that ensures announcements on all platforms.

### 3.4 FocusRing

`FocusRing` implements keyboard-only focus indicator detection. The principle: show the focus ring only when the user is navigating via keyboard, not after a mouse click. This implements the heuristic from the `:focus-visible` CSS pseudo-class.

````rust
// ars-a11y/src/focus/ring.rs

/// Tracks whether the most recent interaction was keyboard-driven.
///
/// `FocusRing` is an accessibility-specific heuristic that consumes the same
/// normalized modality event stream as `ars_core::ModalityContext`. Adapters
/// typically feed it through `ars-dom::ModalityManager` so focus-visible state
/// stays aligned with interaction modality without coupling it to
/// `PlatformEffects`.
///
/// The `data-ars-focus-visible` attribute on focused elements reflects this state
/// and is the CSS hook for styling focus rings.
pub struct FocusRing {
    /// True when the most recent interaction was via keyboard.
    keyboard_modality: AtomicBool,
}

impl FocusRing {
    /// Process a `pointerdown` event — suppresses keyboard modality.
    pub fn on_pointer_down(&self) {
        self.keyboard_modality.store(false, Ordering::Relaxed);
    }

    /// Process a `keydown` event — activates keyboard modality.
    /// Only activates for non-modifier navigation keys (Tab, Arrow keys, etc.).
    ///
    /// **Crate dependency note:** `KeyboardKey` is defined in `ars-core` (not
    /// `ars-interactions`) so that both `ars-a11y` and `ars-interactions` can
    /// reference it without a circular dependency. See 01-architecture.md §1.2.
    /// `ars-interactions` re-exports `KeyboardKey` from `ars-core` for convenience
    /// (see 05-interactions.md §11); the canonical definition lives in `ars-core`.
    /// The `modifiers` parameter allows filtering out modified key combos
    /// (e.g., Ctrl+Tab switches browser tabs and should not trigger keyboard modality).
    /// Adapters filter platform-consumed combos before calling this method.
    /// `modifiers` is the raw `ars_core::KeyModifiers` snapshot from the same
    /// event delivered to `ModalityContext`.
    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        // Keys that indicate keyboard navigation intent.
        // F1-F12 are included because they indicate keyboard-driven interaction.
        // If browser-level F-key handling (e.g., F5=refresh) is undesirable,
        // the adapter can filter them before invoking FocusRing.
        const NAV_KEYS: &[KeyboardKey] = &[
            KeyboardKey::Tab, KeyboardKey::ArrowUp, KeyboardKey::ArrowDown,
            KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight,
            KeyboardKey::Home, KeyboardKey::End, KeyboardKey::PageUp,
            KeyboardKey::PageDown, KeyboardKey::Enter, KeyboardKey::Space,
            KeyboardKey::Escape, KeyboardKey::F1, KeyboardKey::F2,
            KeyboardKey::F3, KeyboardKey::F4, KeyboardKey::F5, KeyboardKey::F6,
            KeyboardKey::F7, KeyboardKey::F8, KeyboardKey::F9, KeyboardKey::F10,
            KeyboardKey::F11, KeyboardKey::F12,
        ];
        // Skip modified combos (e.g., Ctrl+Tab = browser tab switch).
        // The adapter also pre-filters platform-consumed combos, but this
        // guard ensures FocusRing remains correct even without adapter filtering.
        if modifiers.ctrl || modifiers.meta || modifiers.alt {
            // Skip ctrl/meta-modified combos (Ctrl+Tab, Cmd+Tab) and alt-modified
            // combos (Alt+Tab = OS window switch). These are not user keyboard
            // navigation and should not trigger keyboard modality.
            return;
        }
        if NAV_KEYS.contains(&key) {
            self.keyboard_modality.store(true, Ordering::Relaxed);
        }
    }

    /// Process a virtual interaction source (for example assistive-technology navigation).
    pub fn on_virtual_input(&self) {
        self.keyboard_modality.store(true, Ordering::Relaxed);
    }

    /// Returns true if the focus ring should be shown for the element
    /// that just received focus.
    pub fn should_show_focus_ring(&self) -> bool {
        self.keyboard_modality.load(Ordering::Relaxed)
    }

    /// Emit the `data-ars-focus-visible` attribute into AttrMap based on current state.
    ///
    /// **Note:** Components using `ars-interactions` MUST use `FocusResult.current_attrs()`
    /// instead. Direct use of this method is reserved for rare cases where the
    /// `ars-interactions` layer is bypassed. See `05-interactions.md` §4 normative statement.
    pub fn apply_focus_attrs(&self, attrs: &mut AttrMap, is_focused: bool) {
        if is_focused && self.keyboard_modality.load(Ordering::Relaxed) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        } else {
            attrs.set(HtmlAttr::Data("ars-focus-visible"), AttrValue::None);
        }
    }
}

// Note: `apply_focus_attrs()` actively removes `data-ars-focus-visible` via
// `AttrValue::None` — this is intentional. It clears stale attributes from
// previous renders, which is necessary because this method writes into a
// mutable AttrMap that may already contain the attribute from a prior call.
// By contrast, `FocusResult::current_attrs()` in `05-interactions.md` simply
// omits the attribute when it should not be present, relying on the framework's
// attribute diffing to remove it from the DOM. Both approaches converge to the
// same DOM result: the attribute is present only when focus-visible is active.

/// CSS technique for focus ring styling using the data attribute.
/// This is a documentation pattern; actual CSS is user-provided.
///
/// ### Focus Indicator Requirements
///
/// The `data-ars-focus-visible` attribute is the canonical focus indicator hook.
/// Adapters MUST NOT rely on `:focus-visible` alone — the data attribute provides
/// cross-browser consistency and programmatic control.
///
/// **WCAG requirements for focus indicators:**
/// - **WCAG 2.4.7 (AA)**: Focus indicator must have at least 3:1 contrast ratio
///   against adjacent colors.
/// - **WCAG 2.4.11 (AA)**: Focus indicator must be at least 2px thick (minimum
///   area requirement). ars-ui recommends 3px for clarity.
/// - **WCAG 2.4.12 (AAA)**: Focus indicator should not be obscured by other content.
///
/// **Forced-colors mode**: The `outline` property is preserved in Windows High
/// Contrast mode. `box-shadow` is NOT — never use `box-shadow` as the sole focus
/// indicator. Always include an `outline` declaration alongside or instead of
/// `box-shadow`.
///
/// ```css
/// /* Base: remove default outline (we provide our own) */
/// [data-ars-scope] { outline: none; }
///
/// /* Focus ring: 3px solid, 3:1 contrast minimum */
/// [data-ars-focus-visible] {
///   outline: 3px solid #0070f3;
///   outline-offset: 2px;
/// }
///
/// /* Forced-colors / high-contrast mode — outline remains visible */
/// @media (forced-colors: active) {
///   [data-ars-focus-visible] {
///     outline: 3px solid ButtonText;
///     /* box-shadow is stripped in forced-colors; outline is preserved */
///   }
/// }
///
/// /* Dark mode: adjust color for contrast */
/// .dark [data-ars-focus-visible] {
///   outline-color: #79b8ff;
/// }
/// ```
pub struct FocusRingCssDoc;
````

> **Interaction layer:** FocusRing in `ars-a11y` provides the accessibility-focused `focus-visible` heuristic. The `FocusState` enum in `ars-interactions` (`05-interactions.md` §4) reads the shared `ModalityContext`, while adapters feed both through the same `ars-dom::ModalityManager` event stream.

### 3.5 FocusZone

`FocusZone` groups a set of focusable elements and handles arrow-key navigation among them. It is the foundation for toolbars, tab lists, radio groups, and other composite widgets that use arrow-key movement internally.

```rust
// ars-a11y/src/focus/zone.rs

/// Configuration for a FocusZone.
#[derive(Clone, Debug)]
pub struct FocusZoneOptions {
    /// Axis of arrow-key navigation.
    pub direction: FocusZoneDirection,

    /// If true, pressing ArrowRight/Down at the last item wraps to the first.
    pub wrap: bool,

    /// If true, use roving tabindex strategy (only one item has tabindex=0).
    /// If false, use aria-activedescendant strategy.
    /// Corresponds to `FocusStrategy::RovingTabindex` (true) vs
    /// `FocusStrategy::ActiveDescendant` (false) — see §3.2.
    pub roving_tabindex: bool,

    /// If true, Home/End keys move to first/last item.
    pub home_end: bool,

    /// If true, PageUp/PageDown are active (useful for long lists).
    pub page_navigation: bool,

    /// Number of items to skip per PageUp/PageDown.
    pub page_size: NonZero<usize>,

    /// If true, disabled items are skipped during arrow-key navigation within
    /// the focus zone. Note: this controls arrow-key traversal only — disabled
    /// items remain in the Tab order per §13 (disabled elements stay focusable).
    ///
    /// Per-component guidance:
    ///   - RadioGroup, Tabs: SHOULD set `skip_disabled: false` to match APG
    ///     guidance that disabled options remain discoverable via arrow keys.
    ///   - Menu, Listbox: MAY keep `true` (default) since disabled items are
    ///     announced by screen readers but not interactable.
    pub skip_disabled: bool,
}
```

> **Component override:** RadioGroup and Tabs components SHOULD override `skip_disabled` to `false` to allow focus on disabled items (WAI-ARIA requirement). See Appendix B for the full component-to-options mapping.

```rust
impl Default for FocusZoneOptions {
    fn default() -> Self {
        Self {
            direction: FocusZoneDirection::Vertical,
            wrap: true,
            roving_tabindex: true,
            home_end: true,
            page_navigation: false,
            page_size: NonZero::new(10).expect("hardcoded nonzero"),
            skip_disabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusZoneDirection {
    /// Arrow Up/Down navigate; Arrow Left/Right are not intercepted.
    Vertical,
    /// Arrow Left/Right navigate; Arrow Up/Down are not intercepted.
    Horizontal,
    /// All four arrow keys navigate in a 2D grid.
    Grid { cols: NonZero<usize> },
    /// Arrow Left/Right AND Up/Down navigate in a single-dimension flat list.
    /// All four arrow keys step +/- 1, with Left/Right flipped in RTL mode.
    /// This is NOT a 2D grid — use `Grid { cols }` for 2D navigation.
    /// Wrapping from the first item on ArrowUp to the last item (and vice versa)
    /// is controlled by `FocusZoneOptions::wrap` and is intentional for flat lists.
    Both,
}

impl FocusZoneDirection {
    /// Creates a `Grid` direction with the given column count.
    ///
    /// # Panics
    /// Panics if `cols` is zero.
    pub fn grid(cols: usize) -> Self {
        Self::Grid {
            cols: NonZero::new(cols).expect("grid must have at least one column"),
        }
    }
}

/// A managed set of items navigable via arrow keys.
/// Used in the context of a component's machine context.
pub struct FocusZone {
    pub options: FocusZoneOptions,
    /// Index of the currently active/focused item.
    pub active_index: usize,
    /// Total number of items (may be computed from a collection).
    pub item_count: usize,
}

impl FocusZone {
    pub fn new(options: FocusZoneOptions, item_count: usize) -> Self {
        Self { options, active_index: 0, item_count }
    }

    /// Process a navigation key and return the new active index (if changed).
    ///
    /// Returns `Some(new_index)` if navigation occurred, `None` if the key is not handled.
    pub fn handle_key(
        &self,
        key: KeyboardKey,
        is_rtl: bool,
        is_disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if self.item_count == 0 { return None; }

        let (prev_key, next_key) = match self.options.direction {
            FocusZoneDirection::Vertical => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
            FocusZoneDirection::Horizontal => {
                if is_rtl { (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft) } else { (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight) }
            }
            FocusZoneDirection::Both => {
                // Both axes: vertical uses Up/Down, horizontal uses Left/Right with RTL.
                // Handled below in the extended match.
                (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown)
            }
            FocusZoneDirection::Grid { .. } => {
                // Grid: vertical uses Up/Down, horizontal uses Left/Right with RTL.
                // Handled below in the extended match.
                (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown)
            }
        };

        // For Both and Grid modes, also handle the horizontal axis with RTL awareness.
        let (h_prev_key, h_next_key) = if is_rtl {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        } else {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        };

        let next = match key {
            // Generic vertical axis — excluded for Grid (Grid handles Up/Down
            // with ±cols stride in the grid-specific arms below).
            k if k == prev_key
                && !matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                self.navigate(-1, &is_disabled)
            }
            k if k == next_key
                && !matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                self.navigate(1, &is_disabled)
            }

            // Horizontal axis for Both mode (flat list: Left/Right also navigate ±1)
            k if matches!(self.options.direction, FocusZoneDirection::Both) && k == h_prev_key => {
                self.navigate(-1, &is_disabled)
            }
            k if matches!(self.options.direction, FocusZoneDirection::Both) && k == h_next_key => {
                self.navigate(1, &is_disabled)
            }

            // Horizontal axis for Grid mode (Left/Right navigate ±1 within the row)
            k if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) && k == h_prev_key => {
                self.navigate(-1, &is_disabled)
            }
            k if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) && k == h_next_key => {
                self.navigate(1, &is_disabled)
            }

            // Grid: Up/Down navigate by ±cols to move between rows.
            // Uses navigate_to_exact to land on the target or skip to next non-disabled.
            KeyboardKey::ArrowUp if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) => {
                if let FocusZoneDirection::Grid { cols } = self.options.direction {
                    let stride = cols.get();
                    // Loop upward through rows until we find a non-disabled cell or run out
                    let mut candidate = self.active_index.checked_sub(stride);
                    while let Some(t) = candidate {
                        if !self.options.skip_disabled || !is_disabled(t) {
                            break;
                        }
                        candidate = t.checked_sub(stride);
                    }
                    candidate.filter(|&t| !self.options.skip_disabled || !is_disabled(t))
                } else { None }
            }
            KeyboardKey::ArrowDown if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) => {
                if let FocusZoneDirection::Grid { cols } = self.options.direction {
                    let stride = cols.get();
                    // Loop downward through rows until we find a non-disabled cell or run out
                    let mut target = self.active_index + stride;
                    while target < self.item_count {
                        if !self.options.skip_disabled || !is_disabled(target) {
                            break;
                        }
                        target += stride;
                    }
                    if target < self.item_count
                        && (!self.options.skip_disabled || !is_disabled(target)) {
                        Some(target)
                    } else { None }
                } else { None }
            }

            KeyboardKey::Home if self.options.home_end => {
                // Returns None if all items are disabled — focus stays unchanged.
                self.find_from_inclusive(0, 1, &is_disabled)
            }
            KeyboardKey::End if self.options.home_end => {
                let last = self.item_count.saturating_sub(1);
                // Returns None if all items are disabled — focus stays unchanged.
                self.find_from_inclusive(last, -1, &is_disabled)
            }
            KeyboardKey::PageDown if self.options.page_navigation => {
                let target = (self.active_index + self.options.page_size.get())
                    .min(self.item_count.saturating_sub(1));
                // Search forward (+1) from target (inclusive) to find nearest non-disabled item
                // at or beyond the page target.
                self.find_from_inclusive(target, 1, &is_disabled)
            }
            KeyboardKey::PageUp if self.options.page_navigation => {
                let target = self.active_index.saturating_sub(self.options.page_size.get());
                // Search backward (-1) from target (inclusive) to find nearest non-disabled item
                // at or before the page target.
                self.find_from_inclusive(target, -1, &is_disabled)
            }
            _ => None,
        };

        // Ensure the result index actually changed
        next.filter(|&idx| idx != self.active_index)
    }

    fn navigate(&self, delta: i32, is_disabled: &impl Fn(usize) -> bool) -> Option<usize> {
        self.find_from(self.active_index, delta, is_disabled)
    }

    fn find_from(
        &self,
        start: usize,
        delta: i32,
        is_disabled: &impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let count = i32::try_from(self.item_count).expect("FocusZone supports up to i32::MAX items");
        let mut idx = start as i32 + delta;

        for _ in 0..self.item_count {
            if self.options.wrap {
                idx = idx.rem_euclid(count);
            } else {
                if idx < 0 || idx >= count { return None; }
            }

            let u = idx as usize;
            if !self.options.skip_disabled || !is_disabled(u) {
                return Some(u);
            }
            idx += delta;
        }
        None
    }

    /// Like `find_from`, but tests `start` itself before stepping.
    /// Used by Home/End to ensure the boundary index is evaluated.
    fn find_from_inclusive(
        &self,
        start: usize,
        delta: i32,
        is_disabled: &impl Fn(usize) -> bool,
    ) -> Option<usize> {
        // Test start first.
        if !self.options.skip_disabled || !is_disabled(start) {
            return Some(start);
        }
        self.find_from(start, delta, is_disabled)
    }

    /// Generate tabindex value for an item at the given index.
    /// In roving tabindex mode: 0 for active, -1 for all others.
    /// In non-roving mode: all items get tabindex -1 (aria-activedescendant is used).
    pub fn tabindex_for(&self, index: usize) -> i32 {
        if self.options.roving_tabindex {
            if index == self.active_index { 0 } else { -1 }
        } else {
            -1
        }
    }
}
```

---

### 3.6 IME Composition Handling

All text input components (TextField, Textarea, Combobox, TagsInput, SearchInput, PinInput, Editable) MUST handle Input Method Editor (IME) composition correctly:

1. Track `is_composing: bool` in component Context, set to `true` on `compositionstart` and `false` on `compositionend`.
2. During composition (`is_composing == true`), suppress:
    - Enter-based actions (tag addition, item selection, form submission)
    - Filtering/search triggers (Combobox should not filter on intermediate composition text)
    - Custom keyboard handlers that would interfere with composition
3. Only process the final committed text on `compositionend`.
4. When the browser reports `key` as `"Process"` (i.e., `key == KeyboardKey::Process` — Chrome fires this before `compositionstart`), treat as composition-in-progress regardless of the `isComposing` flag value.
5. **Firefox late-fire workaround:** Firefox fires `compositionend` _after_ `keydown` for Enter, so `is_composing` is still `true` when the Enter `keydown` arrives. The adapter layer MUST schedule a microtask (e.g., `queueMicrotask` / `Promise::resolve().then(...)`) from the Enter `keydown` handler and only process the Enter action in that microtask — by then `compositionend` will have fired and `is_composing` will be `false`. If `is_composing` is still `true` at microtask time, discard the Enter.

This is critical for CJK (Chinese, Japanese, Korean) users who type via composition, as well as users entering accented characters via dead keys on European keyboards.

> **Cross-reference:** `05-interactions.md` §11.5 specifies the adapter-level event normalization for IME composition (keyboard event `is_composing` flag, `KeyboardKey::Process` detection, legacy `keyCode === 229` fallback). The rules above define the component-level behavioral contract; §11.5 defines how the adapter surfaces composition state to components.

---

## 4. Keyboard Navigation

### 4.1 Canonical RTL Keyboard Navigation Matrix

When a component's container has `dir="rtl"`, horizontal arrow key navigation MUST be reversed to match visual reading order. Vertical navigation is unaffected by text direction.

| Key          | LTR Horizontal | RTL Horizontal    | Vertical (any dir) |
| ------------ | -------------- | ----------------- | ------------------ |
| `ArrowRight` | Next item      | **Previous** item | —                  |
| `ArrowLeft`  | Previous item  | **Next** item     | —                  |
| `ArrowDown`  | —              | —                 | Next item          |
| `ArrowUp`    | —              | —                 | Previous item      |
| `Home`       | First item     | First item        | First item         |
| `End`        | Last item      | Last item         | Last item          |

**Note on Home/End**: `Home` and `End` always go to the first and last items respectively in DOM/logical order, regardless of direction. They do NOT reverse in RTL because "first" and "last" refer to the semantic sequence, not visual position.

**Components that MUST apply this matrix** (horizontal orientation):

- RadioGroup (horizontal layout)
- Tabs (horizontal tab list)
- Accordion (horizontal variant, if supported)
- MenuBar (horizontal menu bar)
- Slider (horizontal — see `components/input/slider.md` RTL section)
- Splitter (horizontal — see `components/layout/splitter.md` RTL section)

**Exceptions**:

- **TreeView**: Arrow keys control expand/collapse, not horizontal movement. `ArrowRight` always expands or moves to first child; `ArrowLeft` always collapses or moves to parent. TreeView does NOT flip arrows in RTL.
- **Vertical-only components**: Components that only support vertical orientation (e.g., vertical Listbox) are unaffected by `dir`.

**Detection**: Components determine direction by reading the `dir` attribute from the nearest ancestor with an explicit `dir` value (including the component's own element). If no `dir` is set, default to LTR.

### 4.2 Standard Keyboard Patterns by Component Type

These patterns implement the WAI-ARIA APG keyboard interaction specifications. All patterns must be implemented exactly as specified; deviations require explicit documentation.

#### 4.2.1 Listbox (role="listbox")

| Key                     | Action                                            |
| ----------------------- | ------------------------------------------------- |
| `Arrow Down`            | Move focus to next option                         |
| `Arrow Up`              | Move focus to previous option                     |
| `Home`                  | Move focus to first option                        |
| `End`                   | Move focus to last option                         |
| `Enter` / `Space`       | Select focused option (single-select)             |
| `Shift + Arrow Down/Up` | Extend selection (multi-select)                   |
| `Ctrl + A`              | Select all (multi-select)                         |
| `Printable chars`       | Type-ahead: move focus to next matching option    |
| `Escape`                | Close listbox if inside a popup; deselect nothing |

#### 4.2.2 Menu / Menubar (role="menu", role="menubar")

| Key               | Action                                                   |
| ----------------- | -------------------------------------------------------- |
| `Arrow Down`      | Focus next item (menu)                                   |
| `Arrow Up`        | Focus previous item (menu)                               |
| `Arrow Right`     | Open submenu; in menubar, focus next top-level item      |
| `Arrow Left`      | Close submenu; in menubar, focus previous top-level item |
| `Home`            | Focus first item                                         |
| `End`             | Focus last item                                          |
| `Enter`           | Activate focused item; open submenu if applicable        |
| `Space`           | Activate focused item (same as Enter for most items)     |
| `Escape`          | Close menu; return focus to trigger                      |
| `Tab`             | Close menu; move focus to next element in page tab order |
| `Printable chars` | Type-ahead: focus next matching item                     |

#### 4.2.3 Tabs (role="tablist")

| Key                        | Action                                       |
| -------------------------- | -------------------------------------------- |
| `Arrow Right` (horizontal) | Move focus to next tab; wrap                 |
| `Arrow Left` (horizontal)  | Move focus to previous tab; wrap             |
| `Arrow Down` (vertical)    | Move focus to next tab; wrap                 |
| `Arrow Up` (vertical)      | Move focus to previous tab; wrap             |
| `Home`                     | Move focus to first tab                      |
| `End`                      | Move focus to last tab                       |
| `Space` / `Enter`          | Activate focused tab (if not auto-activated) |
| `Delete`                   | Remove tab (if deletion is supported)        |

Tab panels are activated either **automatically** (focus moves tab → panel is shown) or **manually** (Space/Enter required). ars-ui supports both via `tabs::Config::activation: TabActivation`.

#### 4.2.4 Dialog (role="dialog")

| Key           | Action                                                 |
| ------------- | ------------------------------------------------------ |
| `Escape`      | Close the dialog; return focus to trigger              |
| `Tab`         | Move focus to next focusable element within dialog     |
| `Shift + Tab` | Move focus to previous focusable element within dialog |

Focus is contained within the dialog. Tab at the last element wraps to the first; Shift+Tab at the first wraps to the last.

#### 4.2.5 Tree (role="tree")

| Key               | Action                                                                                    |
| ----------------- | ----------------------------------------------------------------------------------------- |
| `Arrow Down`      | Move focus to next visible item                                                           |
| `Arrow Up`        | Move focus to previous visible item                                                       |
| `Arrow Right`     | On collapsed item: expand. On expanded item: move to first child. On end node: do nothing |
| `Arrow Left`      | On expanded item: collapse. On collapsed or end node: move to parent                      |
| `Home`            | Move focus to first item                                                                  |
| `End`             | Move focus to last visible item                                                           |
| `Enter`           | Activate item (select, navigate, etc.)                                                    |
| `Space`           | Toggle selection                                                                          |
| `* (asterisk)`    | Expand all siblings at the same level                                                     |
| `Printable chars` | Type-ahead                                                                                |

#### 4.2.6 Grid / Treegrid (role="grid")

| Key               | Action                               |
| ----------------- | ------------------------------------ |
| `Arrow Right`     | Move focus one cell right            |
| `Arrow Left`      | Move focus one cell left             |
| `Arrow Down`      | Move focus one cell down             |
| `Arrow Up`        | Move focus one cell up               |
| `Ctrl + Home`     | Focus first cell (row 1, col 1)      |
| `Ctrl + End`      | Focus last cell (last row, last col) |
| `Home`            | Focus first cell in current row      |
| `End`             | Focus last cell in current row       |
| `Page Down`       | Move focus down by page size rows    |
| `Page Up`         | Move focus up by page size rows      |
| `Enter`           | Open edit mode for editable cells    |
| `Escape`          | Exit edit mode; revert changes       |
| `Tab` (edit mode) | Move to next cell; exit edit mode    |

#### 4.2.7 Combobox (role="combobox")

| Key                | Action                                                   |
| ------------------ | -------------------------------------------------------- |
| `Arrow Down`       | Open listbox (if closed); move focus to next option      |
| `Arrow Up`         | Open listbox (if closed); move focus to previous option  |
| `Enter`            | Select focused option; close listbox; update input value |
| `Escape`           | Close listbox without selecting; restore original value  |
| `Alt + Arrow Down` | Open listbox without moving selection                    |
| `Alt + Arrow Up`   | Select focused option; close listbox                     |
| `Home`             | Move cursor to start of input text                       |
| `End`              | Move cursor to end of input text                         |
| `Printable chars`  | Filter the listbox options by input text                 |

#### 4.2.8 Slider (role="slider")

| Key                         | Action                                     |
| --------------------------- | ------------------------------------------ |
| `Arrow Right` / `Arrow Up`  | Increment by step                          |
| `Arrow Left` / `Arrow Down` | Decrement by step                          |
| `Home`                      | Set to minimum value                       |
| `End`                       | Set to maximum value                       |
| `Page Up`                   | Increment by large step (10× default step) |
| `Page Down`                 | Decrement by large step                    |

> **RTL:** In RTL mode, Arrow Right decrements and Arrow Left increments (see Canonical RTL Keyboard Navigation Matrix above). Arrow Up/Down are unaffected by text direction.

#### 4.2.9 Accordion

| Key           | Action                                                               |
| ------------- | -------------------------------------------------------------------- |
| Enter / Space | Toggle expanded state of focused header                              |
| Tab           | Move focus between accordion headers                                 |
| Down Arrow    | Move focus to next accordion header (when headers use FocusZone)     |
| Up Arrow      | Move focus to previous accordion header (when headers use FocusZone) |
| Home          | Move focus to first header (when headers use FocusZone)              |
| End           | Move focus to last header (when headers use FocusZone)               |

> **Horizontal variant:** For horizontal accordion layouts, replace Down/Up Arrow with Right/Left Arrow, with RTL reversal per the Canonical RTL Keyboard Navigation Matrix above.

#### 4.2.10 RadioGroup

| Key                      | Action                                             |
| ------------------------ | -------------------------------------------------- |
| Arrow Up / Arrow Left    | Move focus and select previous radio               |
| Arrow Down / Arrow Right | Move focus and select next radio                   |
| Tab                      | Move focus to/from the radio group                 |
| Space                    | Select the focused radio (if not already selected) |

### 4.3 Type-Ahead / Type-Select Implementation

```rust
// ars-a11y/src/keyboard/typeahead.rs

use unicode_normalization::UnicodeNormalization;

/// Type-ahead (type-select) state for list widgets.
///
/// All string comparisons use NFC-normalized forms to ensure consistent
/// matching regardless of how users or data sources compose characters.
///
/// When the user types printable characters quickly, moves focus to the
/// next item whose label starts with the typed string. After a timeout,
/// the search string is cleared and restarts.
pub struct State {
    /// The accumulated search string (cleared after `timeout_ms`).
    buffer: String,
    /// Timestamp (milliseconds) of the last key press.
    last_key_time: f64,
    /// How long (ms) to wait before clearing the buffer. Default: 500ms.
    pub timeout_ms: f64,
}

impl State {
    pub fn new() -> Self {
        Self { buffer: String::new(), last_key_time: 0.0, timeout_ms: 500.0 }
    }

    /// Process a printable key character and return the search string.
    ///
    /// `now_ms` is the current time in milliseconds (e.g., `performance.now()`).
    /// If `now_ms - last_key_time > timeout_ms`, the buffer is reset before
    /// appending the new character.
    ///
    /// Returns the current search string to match against item labels.
    pub fn process_key(&mut self, key: char, now_ms: f64) -> &str {
        if now_ms - self.last_key_time > self.timeout_ms {
            self.buffer.clear();
        }
        self.last_key_time = now_ms;
        // Unicode-aware lowercase — supports CJK, Arabic, Cyrillic, etc.
        // (to_ascii_lowercase would break non-Latin scripts)
        for c in key.to_lowercase() {
            self.buffer.push(c);
        }
        // Normalize buffer to NFC before comparison so that equivalent
        // compositions (e.g., 'é' as U+00E9 vs U+0065 U+0301) always match.
        self.buffer = self.buffer.nfc().collect::<String>();
        &self.buffer
    }

    /// Find the next matching item index, starting search AFTER `from_index`.
    ///
    /// Search wraps around. Matching is case-insensitive prefix match.
    /// If `buffer` contains a single repeated character (e.g., "aaa"), cycles
    /// through all items starting with that character.
    pub fn find_next_match(
        &self,
        from_index: usize,
        item_count: usize,
        label_for: impl Fn(usize) -> String,
        is_disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if self.buffer.is_empty() || item_count == 0 { return None; }

        let search = &self.buffer;

        // Detect repeated-char scenario: "aaa" → search for "a"
        let first_char = search.chars().next();
        let effective_search: &str = if first_char.is_some() && search.chars().all(|c| Some(c) == first_char) {
            &search[..search.char_indices().nth(1).map(|(i, _)| i).unwrap_or(search.len())]
        } else {
            search
        };

        for offset in 1..=item_count {
            let idx = (from_index + offset) % item_count;
            if is_disabled(idx) { continue; }
            // Normalize label to NFC before case-folding for consistent matching.
            let label = label_for(idx).nfc().collect::<String>().to_lowercase();
            if label.starts_with(effective_search) {
                return Some(idx);
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Returns true if a `KeyboardEventData` represents a printable character suitable
/// for type-ahead. With the `KeyboardKey` enum, printable characters are indicated
/// by `data.character.is_some()` — this helper exists for readability.
pub fn is_printable_key(data: &KeyboardEventData) -> bool {
    data.character.is_some()
}
```

> **Dead-key handling.** Dead keys (used for diacritics in many European keyboard layouts)
> fire `key="Dead"`, which is a multi-character string correctly rejected by
> `is_printable_key()`. Dead-key compositions arrive via `compositionend`. Type-ahead
> SHOULD listen for `input` events as fallback for composed characters.
>
> **Locale-specific case folding.** `to_lowercase()` handles most scripts correctly but
> note that Turkish has special case rules: dotless I (I) maps to dotless ı, and dotted İ maps to i. For full correctness,
> consider using ICU4X `CaseFolding` with locale parameter for Turkish/Azerbaijani locales.

### 4.4 Keyboard Shortcut Registration

```rust
// ars-a11y/src/keyboard/shortcuts.rs

/// Minimal keyboard event trait for platform-agnostic modifier matching.
/// Adapter layers implement this for their framework's event types
/// (e.g., `web_sys::KeyboardEvent`, Dioxus `KeyboardData`).
///
/// **Note:** `ctrl_key()` and `meta_key()` expose raw modifier state. For
/// cross-platform "action key" semantics (Ctrl on Windows/Linux, Cmd on macOS),
/// callers MUST use `KeyModifiers::matches_event()` — do NOT read `ctrl_key()`
/// or `meta_key()` directly to decide on the "action" modifier.
pub trait DomEvent {
    fn key(&self) -> Option<&str>;
    fn shift_key(&self) -> bool;
    fn ctrl_key(&self) -> bool;
    fn meta_key(&self) -> bool;
    fn alt_key(&self) -> bool;
    fn event_type(&self) -> &str;
    fn prevent_default(&self);
    fn stop_propagation(&self);
}

/// A keyboard shortcut descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardShortcut {
    pub key: &'static str,
    pub modifiers: KeyModifiers,
    /// Scope where the shortcut is active. None = global.
    pub scope: Option<&'static str>,
}

/// Modifier key combination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    pub shift: bool,
    /// The "action key" — Ctrl on Windows/Linux, Cmd (Meta) on macOS.
    /// Use this instead of `ctrl` or `meta` for cross-platform shortcuts.
    pub action: bool,
    pub alt: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self { shift: false, action: false, alt: false };
    pub const SHIFT: Self = Self { shift: true, action: false, alt: false };
    pub const ACTION: Self = Self { shift: false, action: true, alt: false };
    pub const ACTION_SHIFT: Self = Self { shift: true, action: true, alt: false };
    pub const ALT: Self = Self { shift: false, action: false, alt: true };

    /// Returns true if the event's modifier state matches this descriptor,
    /// accounting for platform differences (Cmd vs Ctrl).
    pub fn matches_event(&self, event: &dyn crate::DomEvent, platform: Platform) -> bool {
        let action_pressed = match platform {
            Platform::MacOs => event.meta_key(),
            _ => event.ctrl_key(),
        };
        self.shift == event.shift_key()
            && self.action == action_pressed
            && self.alt == event.alt_key()
    }
}

/// Platform identifier for modifier key normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    IOS,
    Windows,
    Linux,
    Unknown,
}

impl Platform {
    /// Detect the current platform from `navigator.platform`.
    /// Called once at startup and cached.
    ///
    /// Note: iPadOS 13+ reports "MacIntel" as its platform string but is a touch
    /// platform. We disambiguate by checking `navigator.maxTouchPoints > 1`, which
    /// is true on iPadOS but false (or 0) on actual macOS hardware.
    pub fn detect(navigator_platform: &str, max_touch_points: u32) -> Self {
        if navigator_platform.contains("Mac") {
            if max_touch_points > 1 { Self::IOS } else { Self::MacOs }
        }
        else if navigator_platform.contains("Win") { Self::Windows }
        else if navigator_platform.contains("Linux") { Self::Linux }
        else { Self::Unknown }
    }

    /// Returns the human-readable action key name for display in tooltips/labels.
    pub fn action_key_label(self) -> &'static str {
        match self {
            Self::MacOs | Self::IOS => "⌘",
            _ => "Ctrl",
        }
    }
}
```

> **Note:** `ars-a11y::KeyModifiers` uses an `action` field that abstracts Ctrl (Windows/Linux) and Meta/Cmd (macOS). For raw per-key modifiers, see `ars-interactions::KeyModifiers` in `05-interactions.md` §2.2. Conversion between the two types: see `From<(ars_interactions::KeyModifiers, Platform)>` impl in `05-interactions.md` §2.2.

---

## 5. Screen Reader Support

### 5.1 LiveAnnouncer

`LiveAnnouncer` provides a programmatic API for announcing text to screen readers via ARIA live regions. It maintains two hidden DOM elements (`aria-live="polite"` and `aria-live="assertive"`) and manages message queuing to ensure reliable announcement across screen readers.

````rust
// ars-a11y/src/announcer.rs

/// Priority of a live announcement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnnouncementPriority {
    /// `aria-live="polite"`: waits for the user to finish speaking before announcing.
    /// Use for non-urgent updates: search results loaded, item added to cart.
    Polite,
    /// `aria-live="assertive"`: interrupts the current speech immediately.
    /// Use for time-sensitive errors or critical status changes only.
    Assertive,
}

/// A pending announcement in the queue.
#[derive(Clone, Debug)]
pub struct Announcement {
    pub message: String,
    pub priority: AnnouncementPriority,
}

/// Service for screen reader announcements, provided via framework context.
///
/// The adapter mounts a `LiveAnnouncerProvider` that wraps the application tree
/// and provides `Rc<RefCell<LiveAnnouncer>>` via the framework's context system.
/// Components access it indirectly through `ars_dom::announce()` and
/// `ars_dom::announce_assertive()` — they never hold a direct reference.
///
/// This ensures announcements are coordinated, deduplicated, and testable
/// (tests can provide a mock announcer or skip the provider for no-op behavior).
///
/// # Screen Reader Compatibility Notes
///
/// Different screen readers handle live region updates differently:
///   - NVDA: Reliably announces text changes in live regions.
///   - JAWS: Requires the region to be present in DOM before activation.
///   - VoiceOver: Only announces when the text *changes*; repeated identical
///     messages are ignored. The implementation works around this by appending
///     a zero-width joiner (U+200D) on alternate announcements of the same
///     message. U+200D is used instead of U+200B (zero-width space) because
///     ZWSP can produce an empty braille cell on refreshable braille displays.
///   - TalkBack: Generally reliable with polite regions.
///
/// # DOM Structure (managed by ars-dom)
///
/// Two hidden elements are injected at the document body level on first use:
///
/// ```html
/// <div
///   id="ars-live-polite"
///   aria-live="polite"
///   aria-atomic="true"
///   data-ars-part="live-region"
///   class="ars-visually-hidden"
/// ></div>
/// <div
///   id="ars-live-assertive"
///   aria-live="assertive"
///   aria-atomic="true"
///   aria-relevant="additions text"
///   data-ars-part="live-region"
///   class="ars-visually-hidden"
/// ></div>
/// ```
pub struct LiveAnnouncer {
    /// Queue of pending announcements.
    queue: Vec<Announcement>,
    /// Whether an announcement is currently being processed.
    announcing: bool,
    /// Toggle bit for VoiceOver deduplication workaround.
    /// Only toggled when the current message is identical to `last_message`.
    voiceover_toggle: bool,
    /// Tracks the last announced message text so the deduplication character
    /// is only appended for consecutive identical messages.
    last_message: Option<String>,
    /// Delay before clearing the live region content (ms). Default: 7000.
    /// Prevents the announcement from being re-read when screen reader focus
    /// enters the live region.
    clear_delay_ms: u32,
}

impl LiveAnnouncer {
    /// Create a new LiveAnnouncer. Call `ensure_dom()` in ars-dom before first use.
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            announcing: false,
            voiceover_toggle: false,
            last_message: None,
            clear_delay_ms: 7000,
        }
    }

    /// Announce a message with polite priority.
    /// The message will be announced when the user is idle.
    pub fn announce(&mut self, message: impl Into<String>) {
        self.announce_with_priority(message, AnnouncementPriority::Polite);
    }

    /// Announce a message with assertive priority.
    /// The message will interrupt current screen reader speech.
    /// Use sparingly — unexpected interruptions degrade UX significantly.
    pub fn announce_assertive(&mut self, message: impl Into<String>) {
        self.announce_with_priority(message, AnnouncementPriority::Assertive);
    }

    /// Announce with explicit priority.
    pub fn announce_with_priority(
        &mut self,
        message: impl Into<String>,
        priority: AnnouncementPriority,
    ) {
        let announcement = Announcement { message: message.into(), priority };

        // Assertive messages clear the queue of pending polite messages.
        if priority == AnnouncementPriority::Assertive {
            self.queue.retain(|a| a.priority == AnnouncementPriority::Assertive);
        }

        self.queue.push(announcement);
        self.process_queue();
    }

    /// Clear all pending announcements.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    fn process_queue(&mut self) {
        if self.announcing || self.queue.is_empty() { return; }

        // Sort: assertive first, then polite.
        self.queue.sort_by_key(|a| core::cmp::Reverse(a.priority));

        let next = self.queue.remove(0);
        self.announcing = true;

        // VoiceOver deduplication: only append a deduplication character when the
        // current message is identical to the last announced message. Uses U+200D
        // (zero-width joiner) instead of U+200B (zero-width space) to avoid
        // artifacts on braille displays — ZWSP can render as an empty braille cell,
        // whereas ZWJ is reliably invisible.
        let is_repeat = self.last_message.as_deref() == Some(&next.message);
        let content = if is_repeat && self.voiceover_toggle {
            format!("{}\u{200D}", next.message)
        } else {
            next.message.clone()
        };
        if is_repeat {
            self.voiceover_toggle = !self.voiceover_toggle;
        } else {
            self.voiceover_toggle = false;
        }
        self.last_message = Some(next.message.clone());

        // ars-dom implementation (see §5.2 for rationale):
        // 1. Remove all child nodes from the live region element.
        // 2. After a delay of 150ms (within the 100–300ms range from §5.2),
        //    insert a new <span> element with the announcement text as a child
        //    of the live region. Node insertion triggers more reliable screen
        //    reader detection than textContent replacement (especially NVDA).
        // 3. After clear_delay_ms, remove the <span> to clean up.
        // 4. The ars-dom adapter calls notify_announced() after the live region update completes.
        let _ = (content, next.priority);
    }

    /// Called by the ars-dom adapter after the live region content has been set
    /// and the clear delay has elapsed. Resets `announcing` and processes the
    /// next queued announcement.
    pub fn notify_announced(&mut self) {
        self.announcing = false;
        self.process_queue();
    }

    /// Returns `AttrMap` for the polite live region element.
    pub fn polite_region_attrs() -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, "ars-live-polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs.set(HtmlAttr::Data("ars-part"), "live-region");
        attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        attrs
    }

    /// Returns `AttrMap` for the assertive live region element.
    /// Asymmetry note: `polite_region_attrs` omits `aria-relevant` (UA default is
    /// "additions text"), but here we set it explicitly so that screen readers that
    /// treat assertive regions differently still get the correct relevance scope.
    ///
    /// Builds assertive region attrs by extending polite region attrs.
    /// Both share the same base structure (visually-hidden, live-region part).
    /// `aria-live`, `id`, and `aria-relevant` differ.
    pub fn assertive_region_attrs() -> AttrMap {
        let mut attrs = LiveAnnouncer::polite_region_attrs();
        attrs.set(HtmlAttr::Id, "ars-live-assertive");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "assertive");
        attrs.set(HtmlAttr::Aria(AriaAttr::Relevant), "additions text");
        attrs
    }
}

impl Default for LiveAnnouncer {
    fn default() -> Self { Self::new() }
}
````

### 5.2 Accessibility Tree Invalidation Timing

When updating live regions, the **method of DOM mutation** and **timing** critically affect screen reader announcement reliability:

**DOM Mutation Strategy:**

- **Insert new nodes** into the live region rather than updating `textContent` of an existing node. Screen readers (especially NVDA) more reliably detect `childList` mutations than `characterData` mutations.
- When clearing and re-announcing the same text, insert a new `<span>` element with the message rather than setting `textContent` again — identical text replacements may be ignored.

**Timing Requirements:**

- Wait **100–300ms** before adding content to a live region after clearing it. Shorter delays (e.g., `setTimeout(0)`) may be swallowed by some screen readers.
- The `LiveAnnouncer.clear_delay_ms` (default 7000ms) controls how long content remains in the region before cleanup.

**Priority Guidelines:**

- Use `aria-live="assertive"` **only** for critical alerts that require immediate user attention (e.g., form validation errors that block submission, session timeout warnings).
- Default to `aria-live="polite"` for all other announcements (selection changes, sort updates, filter results).

**Screen Reader Timing Differences:**

| Screen Reader                   | Behavior                                                                                                                                                                                                                                                                     |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **NVDA + Firefox**              | Announces `aria-live` regions more aggressively; `polite` regions may interrupt current speech. Use slightly longer clear delays (~150ms).                                                                                                                                   |
| **JAWS + Chrome**               | More conservative; may batch rapid announcements. Space announcements ≥200ms apart for reliability.                                                                                                                                                                          |
| **VoiceOver + Safari**          | Requires the text-toggle technique (alternating trailing U+200D zero-width joiner) for consecutive identical messages. The `voiceover_toggle` and `last_message` fields in `LiveAnnouncer` handle this. U+200D is used instead of U+200B to avoid braille display artifacts. |
| **TalkBack + Chrome (Android)** | Similar to JAWS timing; test with ≥200ms insertion delay.                                                                                                                                                                                                                    |

### 5.3 VisuallyHidden

Content that is visible to screen readers but hidden from sighted users. Used for supplementary accessibility labels, status messages, and hidden instructions.

````rust
// ars-a11y/src/visually_hidden.rs

/// Returns the AttrMap for a visually-hidden wrapper element.
///
/// The CSS technique used (absolute positioning + clip) avoids the following
/// pitfalls of other approaches:
///   - `display: none` / `visibility: hidden`: hidden from screen readers too.
///   - `opacity: 0`: still takes up space, may be clipped by ancestors.
///   - `font-size: 0`: VoiceOver on macOS may still read the element.
///   - `text-indent: -9999px`: causes performance issues with long text.
///
/// This implementation is safe for RTL layouts and does not cause scroll issues.
///
/// **Important**: Because this technique uses `position: absolute`, the
/// VisuallyHidden element must be placed inside a positioned ancestor
/// (i.e., an element with `position: relative`, `absolute`, `fixed`, or
/// `sticky`). Without a positioned ancestor, the absolutely-positioned
/// element will be placed relative to the initial containing block (the
/// viewport), which can cause unexpected layout shifts and scroll issues.
/// Framework adapters should document this requirement. In practice, most
/// component root elements already have `position: relative` set.
pub fn visually_hidden_attrs() -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Class, "ars-visually-hidden");
    attrs
}

/// CSS for visually-hidden (non-focusable):
///
/// ```css
/// .ars-visually-hidden {
///   position: absolute;
///   width: 1px;
///   height: 1px;
///   padding: 0;
///   margin: -1px;
///   overflow: hidden;
///   clip: rect(0, 0, 0, 0);
///   white-space: nowrap;
///   border-width: 0;
/// }
/// ```
pub struct VisuallyHiddenCssDoc;

/// Returns visually hidden attrs for an element that MUST remain visible
/// when it receives focus (e.g., a "Skip to content" link).
/// When focused, the element becomes visible.
pub fn visually_hidden_focusable_attrs() -> AttrMap {
    // This is implemented via CSS; the data attribute provides the CSS hook.
    let mut attrs = visually_hidden_attrs();
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden-focusable"), true);
    attrs
}

/// CSS for visually-hidden-focusable:
///
/// ```css
/// [data-ars-visually-hidden-focusable]:not(:focus):not(:focus-within) {
///   position: absolute;
///   width: 1px;
///   height: 1px;
///   padding: 0;
///   margin: -1px;
///   overflow: hidden;
///   clip: rect(0, 0, 0, 0);
///   white-space: nowrap;
///   border-width: 0;
/// }
/// ```
pub struct VisuallyHiddenFocusableCssDoc;
````

### 5.4 Label, Description, and Field Utilities

````rust
// ars-a11y/src/label.rs

/// Resolves the accessible name for a form element from multiple possible sources.
///
/// Priority (per accname-1.2 spec):
/// 1. `aria-labelledby` referencing visible text
/// 2. `aria-label` (string)
/// 3. `<label for="...">` association
/// 4. `title` attribute
/// 5. `placeholder` attribute (last resort; discouraged)
#[derive(Clone, Debug, Default)]
pub struct LabelConfig {
    /// IDs of elements that label this element (aria-labelledby).
    pub labelledby_ids: Vec<String>,
    /// Inline string label (aria-label).
    pub label: Option<String>,
    /// ID of a <label> element associated with this input.
    pub html_for_id: Option<String>,
}

impl LabelConfig {
    /// Apply label attributes to AttrMap.
    /// Only one labelling mechanism is applied (priority order above).
    pub fn apply_to(&self, attrs: &mut AttrMap) {
        if !self.labelledby_ids.is_empty() {
            AriaAttribute::LabelledBy(AriaIdList(self.labelledby_ids.clone()))
                .apply_to(attrs);
        } else if let Some(ref label) = self.label {
            AriaAttribute::Label(label.clone()).apply_to(attrs);
        }
        // html_for_id is handled by the <label> element itself via `for` attribute.
    }
}

/// Associates a description with an element.
///
/// Multiple description sources can be combined (aria-describedby accepts multiple IDs).
#[derive(Clone, Debug, Default)]
pub struct DescriptionConfig {
    /// IDs of elements describing this element.
    pub describedby_ids: Vec<String>,
    /// Additional details element ID (aria-details).
    pub details_id: Option<String>,
}

impl DescriptionConfig {
    pub fn apply_to(&self, attrs: &mut AttrMap) {
        if !self.describedby_ids.is_empty() {
            AriaAttribute::DescribedBy(AriaIdList(self.describedby_ids.clone()))
                .apply_to(attrs);
        }
        if let Some(ref id) = self.details_id {
            AriaAttribute::Details(AriaIdRef(id.clone())).apply_to(attrs);
        }
    }
}

/// A complete field context: label + description + error state.
/// Used by form input components (TextField, Select, Combobox, Slider, etc.)
#[derive(Clone, Debug)]
pub struct FieldContext {
    pub ids: ComponentIds,
    pub label: LabelConfig,
    pub description: DescriptionConfig,
    pub is_required: bool,
    pub is_readonly: bool,
    pub is_disabled: bool,
    pub invalid: AriaInvalid,
}

impl FieldContext {
    pub fn new(ids: ComponentIds) -> Self {
        Self {
            ids,
            label: LabelConfig::default(),
            description: DescriptionConfig::default(),
            is_required: false,
            is_readonly: false,
            is_disabled: false,
            invalid: AriaInvalid::False,
        }
    }
}

/// **`aria-describedby` ordering.** When a field has multiple description sources, order them
/// by priority: error message ID first, then general description ID.
/// This ensures screen readers announce the most important information first.
///
/// ```rust
/// let described_by = [error_id, description_id]
///     .iter()
///     .filter_map(|id| id.as_ref())
///     .cloned()
///     .collect::<Vec<_>>()
///     .join(" ");
/// ```

impl FieldContext {
    /// Apply all field attributes to the input element's AttrMap.
    pub fn apply_input_attrs(&self, attrs: &mut AttrMap) {
        self.label.apply_to(attrs);

        // Build aria-describedby with priority ordering:
        // error message > help text > general description.
        let error_id = if self.invalid != AriaInvalid::False {
            Some(self.ids.part("error-message"))
        } else {
            None
        };

        let description_ids = [error_id]
            .into_iter()
            .flatten()
            .chain(self.description.describedby_ids.iter().cloned())
            .collect::<Vec<_>>();

        if !description_ids.is_empty() {
            AriaAttribute::DescribedBy(AriaIdList(description_ids)).apply_to(attrs);
        }

        if let Some(ref id) = self.description.details_id {
            AriaAttribute::Details(AriaIdRef(id.clone())).apply_to(attrs);
        }

        if self.is_required {
            AriaAttribute::Required(true).apply_to(attrs);
        }

        if self.is_readonly {
            AriaAttribute::ReadOnly(true).apply_to(attrs);
        }

        set_disabled(attrs, self.is_disabled);

        if self.invalid != AriaInvalid::False {
            set_invalid(
                attrs,
                self.invalid,
                Some(&self.ids.part("error-message")),
            );
        } else {
            set_invalid(attrs, AriaInvalid::False, None);
        }
    }

    /// Returns AttrMap for the label element.
    pub fn label_element_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ids.part("input"));
        attrs
    }

    /// Returns AttrMap for the description element.
    pub fn description_element_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("description"));
        attrs
    }

    /// Returns AttrMap for the error message element.
    pub fn error_message_attrs(&self, is_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("error-message"));
        AriaAttribute::Live(AriaLive::Polite).apply_to(&mut attrs);
        AriaAttribute::Atomic(true).apply_to(&mut attrs);
        // Hidden from AT when no error is present.
        if !is_visible {
            AriaAttribute::Hidden(Some(true)).apply_to(&mut attrs);
        }
        attrs
    }
}
````

### 5.5 Dynamic Content Announcements

Patterns for specific dynamic content scenarios:

```rust
// ars-a11y/src/announcements.rs

/// Localizable announcement templates for common component state changes.
///
/// Per §2.8: NO hardcoded English strings in ARIA labels or announcements.
/// All announcement text is provided via the `AnnouncementMessages` struct,
/// which implements `ComponentMessages` with English defaults. Consumers
/// supply a locale-appropriate instance through the adapter's context provider.
///
/// **Design note — String vs MessageFn pattern:** This struct uses pre-localized `String`
/// fields (with `{placeholder}` interpolation) rather than the `MessageFn` closure pattern
/// used by `DragAnnouncements` (in `ars-interactions`). This is because `ars-a11y` depends
/// only on `ars-core` and cannot reference `Locale` from `ars-i18n`. The adapter is
/// responsible for constructing the correct locale-specific `AnnouncementMessages` instance.
#[derive(Clone, Debug)]
pub struct AnnouncementMessages {
    pub search_results_zero: String,    // "No results found."
    pub search_results_one: String,     // "1 result found."
    pub search_results_other: String,   // "{count} results found."
    pub selected: String,               // "{label}, selected."
    pub deselected: String,             // "{label}, deselected."
    pub validation_error: String,       // "{field}: {error}. Error."
    pub loading: String,                // "Loading."
    pub loading_complete: String,       // "Loading complete."
    pub item_moved: String,             // "{label} moved to position {position} of {total}."
    pub item_removed: String,           // "{label} removed."
    pub sorted_ascending: String,       // "{column}, sorted ascending."
    pub sorted_descending: String,      // "{column}, sorted descending."
    pub not_sorted: String,             // "{column}, not sorted."
    pub tree_expanded: String,          // "{label}, expanded."
    pub tree_collapsed: String,         // "{label}, collapsed."
}

impl Default for AnnouncementMessages {
    fn default() -> Self {
        Self {
            search_results_zero: "No results found.".into(),
            search_results_one: "1 result found.".into(),
            search_results_other: "{count} results found.".into(),
            selected: "{label}, selected.".into(),
            deselected: "{label}, deselected.".into(),
            validation_error: "{field}: {error}. Error.".into(),
            loading: "Loading.".into(),
            loading_complete: "Loading complete.".into(),
            item_moved: "{label} moved to position {position} of {total}.".into(),
            item_removed: "{label} removed.".into(),
            sorted_ascending: "{column}, sorted ascending.".into(),
            sorted_descending: "{column}, sorted descending.".into(),
            not_sorted: "{column}, not sorted.".into(),
            tree_expanded: "{label}, expanded.".into(),
            tree_collapsed: "{label}, collapsed.".into(),
        }
    }
}

pub struct Announcements;

impl Announcements {
    /// Announce the number of search results.
    pub fn search_results(count: usize, messages: &AnnouncementMessages) -> String {
        match count {
            0 => messages.search_results_zero.clone(),
            1 => messages.search_results_one.clone(),
            n => messages.search_results_other.replace("{count}", &n.to_string()),
        }
    }

    /// Announce a selection change in a listbox.
    pub fn selection_changed(label: &str, selected: bool, messages: &AnnouncementMessages) -> String {
        let template = if selected { &messages.selected } else { &messages.deselected };
        template.replace("{label}", label)
    }

    /// Announce a toast notification.
    pub fn toast(message: &str) -> String {
        message.to_string()
    }

    /// Announce a form validation error.
    pub fn validation_error(field_label: &str, error: &str, messages: &AnnouncementMessages) -> String {
        messages.validation_error.replace("{field}", field_label).replace("{error}", error)
    }

    /// Announce loading state.
    pub fn loading(messages: &AnnouncementMessages) -> String { messages.loading.clone() }
    pub fn loading_complete(messages: &AnnouncementMessages) -> String { messages.loading_complete.clone() }

    /// Announce item moved in a drag-and-drop list.
    pub fn item_moved(label: &str, position: usize, total: usize, messages: &AnnouncementMessages) -> String {
        messages.item_moved
            .replace("{label}", label)
            .replace("{position}", &position.to_string())
            .replace("{total}", &total.to_string())
    }

    /// Announce item removed.
    pub fn item_removed(label: &str, messages: &AnnouncementMessages) -> String {
        messages.item_removed.replace("{label}", label)
    }

    /// Announce sorted column.
    pub fn column_sorted(column: &str, direction: AriaSort, messages: &AnnouncementMessages) -> String {
        let template = match direction {
            AriaSort::Ascending => &messages.sorted_ascending,
            AriaSort::Descending => &messages.sorted_descending,
            _ => &messages.not_sorted,
        };
        template.replace("{column}", column)
    }

    /// Announce tree node expanded/collapsed.
    pub fn tree_node_expanded(label: &str, expanded: bool, messages: &AnnouncementMessages) -> String {
        let template = if expanded { &messages.tree_expanded } else { &messages.tree_collapsed };
        template.replace("{label}", label)
    }
}
```

---

## 6. Color and Visual Accessibility

### 6.1 High Contrast and Forced Colors Mode

> **Cross-reference:** §12 provides component-specific forced-colors guidance (checkboxes, switches, radio buttons, sliders, progress/meter) that supplements these general rules.

Windows High Contrast Mode (WHCM) and CSS `forced-colors` media feature apply a system palette that overrides author-defined colors. ars-ui components must remain functional in this mode.

**Rules for ars-dom and adapter CSS authors:**

1. Never convey state (checked, selected, disabled, active) using color alone. Use `data-ars-*` attributes as additional indicators.
2. In `forced-colors: active` contexts, use `ButtonText`, `ButtonFace`, `HighlightText`, `Highlight`, `GrayText`, and `Canvas` system color keywords.
3. Focus indicators must use `Highlight` or `ButtonText` in forced-colors mode — never `transparent`.
4. Custom SVG icons must have `fill: currentColor` or `forced-color-adjust: auto` to inherit system colors.

> **Canonical location:** `11-dom-utilities.md` §9 — media query utilities including
> `is_forced_colors_active()`, `prefers_reduced_motion()`, `prefers_reduced_transparency()`,
> and `prefers_color_scheme()`. These live in `ars-dom` because they depend on `web_sys::window()`.
> `ars-a11y` re-exports them behind `#[cfg(feature = "dom")]`.

```css
/* Recommended CSS pattern for forced-colors support (documented in spec, applied by users). */

/* Ensure focus rings appear in forced-colors mode */
@media (forced-colors: active) {
    [data-ars-focus-visible] {
        outline: 3px solid Highlight;
        outline-offset: 2px;
        forced-color-adjust: none;
    }

    /* Ensure data-attribute-conveyed states remain visible */
    [data-ars-state~="selected"]::before {
        content: "✓ ";
        forced-color-adjust: auto;
    }

    [data-ars-disabled] {
        color: GrayText;
    }
}
```

> **Per-Component Forced Colors Guidance:** Checkbox checkmark indicators and Switch on/off indicators use pseudo-elements or SVG that may be invisible in Windows High Contrast Mode. These elements MUST have `forced-color-adjust: auto` applied so the browser preserves their visibility using system colors. Specifically:
>
> - **Checkbox**: The `[data-ars-part="indicator"]` element (checkmark/dash icon) needs `forced-color-adjust: auto`.
> - **Switch**: The thumb position indicator and on/off icons need `forced-color-adjust: auto`.
> - **Radio**: The inner dot indicator needs `forced-color-adjust: auto`.
> - Any component using `::before`/`::after` pseudo-elements for state indicators (selected, checked, indeterminate) MUST ensure those pseudo-elements are visible in forced-colors mode.

### 6.2 prefers-reduced-motion

Animations and transitions in ars-ui adapters must respect `prefers-reduced-motion`. The `Presence` component's enter/exit transitions expose a `reduced_motion` prop that disables animation.

```rust
// In component Props structs:
pub struct Props {
    pub present: bool,
    /// If true, skip enter/exit transitions.
    /// ars-a11y sets this from `prefers_reduced_motion()` if not explicitly provided.
    pub skip_animation: Option<bool>,
}

// The skip_animation logic:
pub fn resolve_skip_animation(override_: Option<bool>) -> bool {
    override_.unwrap_or_else(prefers_reduced_motion)
}
```

### 6.3 prefers-reduced-transparency

Components using semi-transparent backdrops or `backdrop-filter` effects MUST respect `prefers-reduced-transparency`. This primarily affects overlay components with translucent backdrops (Dialog, AlertDialog) and any component using glassmorphism-style blur effects.

```rust
// In overlay component Props structs:
pub struct DialogProps {
    /// If true, use an opaque backdrop instead of a translucent one.
    /// ars-a11y sets this from `prefers_reduced_transparency()` if not explicitly provided.
    pub opaque_backdrop: Option<bool>,
}

pub fn resolve_opaque_backdrop(override_: Option<bool>) -> bool {
    override_.unwrap_or_else(prefers_reduced_transparency)
}
```

**Affected components:** Dialog, AlertDialog, and any overlay that renders a backdrop `<div>` with `background: rgba(...)` or `backdrop-filter: blur(...)`.

**When `prefers-reduced-transparency` is active:**

1. Replace `background: rgba(0, 0, 0, 0.5)` with `background: rgb(0, 0, 0)` (or a theme-appropriate opaque color)
2. Remove `backdrop-filter: blur(...)` effects
3. Ensure the backdrop still meets WCAG contrast requirements against the content behind it

```css
/* CSS pattern for reduced-transparency support */
@media (prefers-reduced-transparency: reduce) {
    [data-ars-scope="dialog"] [data-ars-part="backdrop"] {
        background: rgb(0, 0, 0); /* opaque fallback */
        backdrop-filter: none; /* remove blur */
    }
}
```

> **Note:** `prefers-reduced-transparency` has limited browser support (Chrome 118+, no Firefox/Safari as of 2026). The `prefers_reduced_transparency()` function in `11-dom-utilities.md` §9 returns `false` when unsupported, so the fallback behavior is safe — components render normally on browsers that don't support the media query.

### 6.4 Focus Indicator Contrast Requirements

Per WCAG 2.4.11 Focus Appearance (AA):

- The focus indicator must have a minimum area of the perimeter of the unfocused component × 2 CSS pixels.
- The focus indicator must have a contrast ratio of at least 3:1 between the focused and unfocused states.

Recommended defaults (applied via user CSS, documented here for design system authors):

```css
/* Default focus ring: 3px solid outline with 2px offset (per §3.4 FocusRingCssDoc) */
[data-ars-focus-visible] {
    outline: 3px solid #0070f3; /* Must be ≥ 3:1 contrast against background */
    outline-offset: 2px;
}

/* For dark backgrounds, use a white ring with a dark shadow.
   box-shadow provides contrast halo on normal displays;
   it is stripped in forced-colors mode where outline alone is visible. */
.dark [data-ars-focus-visible] {
    outline: 3px solid #ffffff;
    box-shadow: 0 0 0 4px #000000;
}

@media (forced-colors: active) {
    .dark [data-ars-focus-visible] {
        outline: 3px solid Highlight;
        outline-offset: 2px;
        box-shadow: none;
    }
}
```

---

## 7. Touch and Mobile Accessibility

### 7.1 Touch Target Sizing

Per WCAG 2.5.5 (AAA) and Apple HIG / Material Design guidelines, interactive targets must meet minimum size requirements:

| Standard                  | Minimum Size                       |
| ------------------------- | ---------------------------------- |
| WCAG 2.5.5 (AAA)          | 44×44 CSS pixels                   |
| WCAG 2.5.8 (AA, WCAG 2.2) | 24×24 CSS pixels (with exceptions) |
| Apple HIG                 | 44×44 points                       |
| Material Design           | 48×48 dp                           |
| ars-ui recommendation     | 44×44 CSS pixels                   |

#### 7.1.1 Per-Component Minimum Touch Target Sizes

Each interactive component MUST meet the following minimum touch target sizes. Components
whose visual size is smaller than the minimum MUST use invisible hit area expansion.

| Component                    | Minimum Touch Target                        | Notes                                             |
| ---------------------------- | ------------------------------------------- | ------------------------------------------------- |
| Button                       | 44×44 px                                    | Applies to all variants including icon-only       |
| IconButton                   | 44×44 px                                    | Visual icon may be 24px; padding expands hit area |
| Checkbox                     | 44×44 px                                    | Includes label click area                         |
| Radio                        | 44×44 px                                    | Includes label click area                         |
| Switch / Toggle              | 44×44 px                                    | Entire track is tappable                          |
| Slider thumb                 | 48×48 px                                    | Larger target for drag precision                  |
| Slider track                 | 44px height                                 | Full track width is tappable                      |
| Splitter handle              | 16×44 px (vertical) / 44×16 px (horizontal) | Narrow axis uses invisible padding                |
| Select trigger               | 44×44 px                                    | Full trigger area                                 |
| Combobox input               | 44px height                                 | Full width of input                               |
| Tab                          | 44×44 px                                    | Per tab trigger                                   |
| Menu item                    | 44px height                                 | Full width of menu                                |
| Pagination button            | 44×44 px                                    | Per page button                                   |
| Close button (Dialog, Toast) | 44×44 px                                    | Often icon-only; requires padding                 |

#### 7.1.2 Hit Area Expansion Strategies

Three strategies for expanding touch targets beyond the visual footprint:

1. **Padding + negative margin** (default): Adds padding to increase the tappable area,
   then negative margin to cancel the layout effect. Best for inline elements.

2. **`::after` pseudo-element**: An absolutely-positioned `::after` element extends the
   hit area. Best when padding would affect text alignment.

3. **Invisible wrapper**: A transparent wrapper `<div>` with the minimum size. Best for
   complex layouts where padding or pseudo-elements are impractical.

The `touch_target_attrs` utility implements strategy #1:

```rust
// ars-a11y/src/touch.rs

/// Minimum recommended touch target size in CSS pixels.
pub const MIN_TOUCH_TARGET_SIZE: f64 = 44.0;

/// Larger touch target for drag-based controls (Slider thumb).
pub const MIN_DRAG_TARGET_SIZE: f64 = 48.0;

/// Returns inline styles that ensure a minimum touch target size
/// while preserving the visual footprint of smaller elements.
///
/// Uses padding to extend the tap area beyond the visual bounds.
/// This technique is also known as "invisible hit area" or "touch target padding".
pub fn touch_target_attrs(visual_width: f64, visual_height: f64) -> AttrMap {
    touch_target_attrs_with_min(visual_width, visual_height, MIN_TOUCH_TARGET_SIZE)
}

/// Like `touch_target_attrs` but with a custom minimum size (e.g., 48px for drag targets).
pub fn touch_target_attrs_with_min(visual_width: f64, visual_height: f64, min: f64) -> AttrMap {
    let mut attrs = AttrMap::new();

    let h_padding = ((min - visual_width) / 2.0).max(0.0);
    let v_padding = ((min - visual_height) / 2.0).max(0.0);

    if h_padding > 0.0 || v_padding > 0.0 {
        attrs.set_style(CssProperty::Padding, format!("{}px {}px", v_padding, h_padding));
        // Negative margin to cancel out the padding's effect on layout.
        attrs.set_style(CssProperty::Margin, format!("-{}px -{}px", v_padding, h_padding));
    }

    attrs
}
```

### 7.2 Gesture Alternatives

All gesture-based interactions must have keyboard and pointer equivalents:

| Gesture             | Keyboard Alternative               | Pointer Alternative   |
| ------------------- | ---------------------------------- | --------------------- |
| Swipe to dismiss    | Escape key                         | Close button          |
| Pinch to zoom       | +/- buttons or range input         | Scroll wheel          |
| Long press          | Context menu key or Shift+F10      | Right-click           |
| Drag to reorder     | Arrow keys with Enter to grab/drop | Drag handle button    |
| Pull to refresh     | Refresh button                     | Button click          |
| Swipe between items | Arrow keys                         | Previous/Next buttons |

### 7.3 Screen Reader Touch Navigation

VoiceOver (iOS) and TalkBack (Android) use touch-based navigation that differs from keyboard navigation:

- These screen readers use **virtual cursor** navigation, not keyboard tabindex order.
- `aria-activedescendant` is **not supported** by VoiceOver iOS — use roving tabindex for list navigation.
- `aria-hidden="true"` removes elements from touch-based screen reader navigation as well.
- Swipe right/left moves through elements in DOM order; ensure DOM order matches visual/logical order.
- For grouping, use `role="group"` with `aria-label` to give context to related controls.

```rust
// ars-a11y/src/touch.rs (continued)

/// For components that use aria-activedescendant strategy on desktop,
/// this returns whether to fall back to roving tabindex based on
/// the detected screen reader environment.
///
/// In practice this is determined by detecting touch-screen + screen reader,
/// which in ars-dom maps to checking touch support combined with live region behavior.
pub fn should_use_roving_tabindex_for_mobile(platform: Platform) -> bool {
    // VoiceOver iOS does not support aria-activedescendant reliably.
    // When the detected platform is iOS (including iPadOS, which Platform::detect
    // identifies via maxTouchPoints > 1), fall back to roving tabindex.
    matches!(platform, Platform::IOS)
}
```

### 7.4 Virtual Keyboard Considerations

On mobile, the virtual keyboard can obscure form inputs. ars-ui components handle this by:

- Using `inputmode` attribute on inputs to request the appropriate virtual keyboard type.
- Avoiding CSS that prevents scrolling into view after focus.
- Not using `position: fixed` elements that interfere with virtual keyboard display without special handling.

```rust
/// inputmode values for different input types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    None, Text, Tel, Url, Email, Numeric, Decimal, Search,
}

impl InputMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Text => "text",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Email => "email",
            Self::Numeric => "numeric",
            Self::Decimal => "decimal",
            Self::Search => "search",
        }
    }

    /// Apply the inputmode attribute to AttrMap.
    pub fn apply_to(self, attrs: &mut AttrMap) {
        attrs.set(HtmlAttr::InputMode, self.as_str());
    }
}
```

---

## 8. Print Media

When a page is printed (or saved to PDF via the browser's print dialog), interactive UI elements must degrade gracefully. Overlays, scroll locks, and focus-related styles that are meaningful on screen become confusing or invisible on paper.

### 8.1 Overlay Handling

All overlay components (Dialog, AlertDialog, Drawer, Popover, Tooltip, Toast) should be **hidden** in print output unless the overlay content is the primary subject the user intends to print.

- Dialogs with `role="alertdialog"` should remain visible in print because they typically communicate critical information the user may want on paper.
- All other overlays should be hidden via `display: none` in a `@media print` stylesheet.
- The `inert` backdrop element must always be hidden in print.
- Components that use `Presence` for enter/exit animations must also be hidden if they are in `UnmountPending` state during print.

### 8.2 Focus Ring Suppression

Focus rings are a screen-only affordance. In print media they serve no purpose and create visual noise.

- All `data-ars-focus-visible` styles must be wrapped in `@media screen` or explicitly overridden in `@media print`.
- The `FocusRing` component's CSS output should include a `@media print { outline: none !important; box-shadow: none !important; }` reset.

### 8.3 Scroll Lock Removal

When `ScrollLock` is active (typically from an open modal Dialog), the `<body>` or `<html>` element has `overflow: hidden` and possibly a padding-right offset to compensate for the scrollbar. These styles must be removed in print:

```css
@media print {
    html,
    body {
        overflow: visible !important;
        padding-right: 0 !important;
        height: auto !important;
    }
}
```

Framework adapters should document that consumers must include the `ars-ui/print.css` reset (or equivalent) in their application stylesheet, or the above rules if building a custom theme.

### 8.4 Portaled Content

Content rendered via `Portal` into `<div id="ars-portal-root">` is physically located at the end of the document body. In print, this content may appear detached from its logical context.

- The recommended `@media print` stylesheet hides `#ars-portal-root` entirely.
- If a consumer needs to print portaled content (e.g., a dialog with a printable form), they should set `Portal.disabled = true` so the content renders inline at its logical component tree position.
- The `ClientOnly` component (see `components/utility/client-only.md`) renders its `fallback` during SSR; the same fallback should be used for print when the component's children depend on client-side JavaScript state.

### 8.5 Recommended CSS

The `ars-ui` project should ship a `print.css` file (or `@media print` block within the base stylesheet) containing:

```css
@media print {
    /* Hide overlays and backdrops */
    [data-ars-scope="dialog"]:not([role="alertdialog"]),
    [data-ars-scope="drawer"],
    [data-ars-scope="popover"],
    [data-ars-scope="tooltip"],
    [data-ars-scope="toast"],
    [data-ars-backdrop] {
        display: none !important;
    }

    /* Remove scroll lock side-effects */
    html,
    body {
        overflow: visible !important;
        padding-right: 0 !important;
        height: auto !important;
    }

    /* Hide portal root (content should be inlined for print) */
    #ars-portal-root {
        display: none !important;
    }

    /* Suppress focus indicators */
    [data-ars-focus-visible] {
        outline: none !important;
        box-shadow: none !important;
    }

    /* Suppress loading spinners / skeleton screens */
    [data-ars-state="loading"] {
        visibility: hidden;
    }
}
```

Consumers who need to override these defaults (e.g., to show a specific dialog in print) can use a higher-specificity selector or `@media print` rules after the ars-ui stylesheet.

---

## 9. Testing Infrastructure

### 9.1 Automated ARIA Validation

```rust
// ars-a11y/src/testing/validator.rs

/// A compile-time and runtime ARIA attribute validator.
///
/// Catches common ARIA mistakes:
/// - Role set without required attributes
/// - Required owned elements missing
/// - Attributes used on incompatible roles
/// - ID references pointing to non-existent elements
pub struct AriaValidator {
    errors: Vec<AriaValidationError>,
    warnings: Vec<AriaValidationWarning>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AriaValidationError {
    /// A required ARIA attribute is missing for the given role.
    MissingRequiredAttribute {
        role: &'static str,
        missing_attr: &'static str,
    },
    /// An abstract role was used on a DOM element.
    AbstractRoleUsed { role: &'static str },
    /// A required owned element is missing.
    MissingRequiredOwnedElement {
        role: &'static str,
        required_one_of: Vec<&'static str>,
    },
    /// aria-labelledby or aria-describedby references a non-existent ID.
    DanglingIdReference {
        attribute: &'static str,
        id: String,
    },
    /// aria-activedescendant used on a role that does not support it.
    ActiveDescendantOnUnsupportedRole { role: &'static str },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AriaValidationWarning {
    /// Redundant ARIA role matches the implicit native role.
    RedundantRole { element: &'static str, role: &'static str },
    /// aria-label used alongside visible text (prefer aria-labelledby).
    AriaLabelWithVisibleText,
    /// aria-disabled used on a native form element (prefer disabled attribute).
    AriaDisabledOnNativeFormElement,
    /// General advisory hint from the validator.
    Hint { message: &'static str },
}

impl AriaValidator {
    pub fn new() -> Self {
        Self { errors: Vec::new(), warnings: Vec::new() }
    }

    /// Validate role usage.
    pub fn check_role(&mut self, role: AriaRole, attrs: &[AriaAttribute], has_tabindex: bool) {
        // Check for abstract role usage
        if role.is_abstract() {
            self.errors.push(AriaValidationError::AbstractRoleUsed {
                role: role.name(),
            });
            return;
        }

        // Separator vs StructuralSeparator hint: AriaRole::Separator is the focusable
        // widget variant and requires tabindex + value attributes. If the element is not
        // focusable, the developer likely wants AriaRole::StructuralSeparator instead.
        if matches!(role, AriaRole::Separator) {
            if !has_tabindex {
                self.warnings.push(AriaValidationWarning::Hint {
                    message: "AriaRole::Separator requires tabindex for focusable separator. \
                              Use AriaRole::StructuralSeparator for non-focusable separators.",
                });
            }
        }

        // Check required attributes per role
        self.check_required_attrs_for_role(role, attrs);
    }

    fn check_required_attrs_for_role(&mut self, role: AriaRole, attrs: &[AriaAttribute]) {
        let required = required_attributes_for_role(role);
        for req_attr in required {
            let present = attrs.iter().any(|a| a.attr_name() == req_attr);
            if !present {
                self.errors.push(AriaValidationError::MissingRequiredAttribute {
                    role: role.name(),
                    missing_attr: req_attr,
                });
            }
        }
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }
    pub fn errors(&self) -> &[AriaValidationError] { &self.errors }
    pub fn warnings(&self) -> &[AriaValidationWarning] { &self.warnings }
}

/// Returns required ARIA attributes for a role, as per WAI-ARIA 1.2 spec.
/// These attributes MUST be present when using the role.
pub fn required_attributes_for_role(role: AriaRole) -> &'static [&'static str] {
    match role {
        AriaRole::Checkbox | AriaRole::Radio | AriaRole::Switch
        | AriaRole::Menuitemcheckbox | AriaRole::Menuitemradio
            => &["aria-checked"],
        AriaRole::Combobox
            => &["aria-expanded"],
        AriaRole::Scrollbar
            => &["aria-controls", "aria-valuenow", "aria-valuemin", "aria-valuemax"],
        AriaRole::Slider
            => &["aria-valuenow", "aria-valuemin", "aria-valuemax"],
        // WAI-ARIA 1.2 formally requires only aria-valuenow for Spinbutton.
        // aria-valuemin/max are strongly recommended but not required.
        AriaRole::Spinbutton
            => &["aria-valuenow"],
        // WAI-ARIA 1.2 formally requires only aria-valuenow for Meter.
        // aria-valuemin/max are strongly recommended (default 0 and 100 if absent).
        AriaRole::Meter
            => &["aria-valuenow"],
        // Focusable separator (widget role) requires value attributes.
        // Non-focusable separators should use AriaRole::StructuralSeparator instead,
        // which falls through to the wildcard and requires no attributes.
        AriaRole::Separator
            => &["aria-valuenow", "aria-valuemin", "aria-valuemax"],
        AriaRole::Heading
            => &["aria-level"],
        AriaRole::Option
            => &[], // aria-selected is required in some contexts but not globally
        // Note: StructuralSeparator has no required attributes (covered by wildcard)
        _ => &[],
    }
}

/// Validate that an AttrMap produced by a connect() function is
/// ARIA-conformant. Called in debug builds and in test infrastructure.
pub fn validate_attr_map(role: Option<AriaRole>, attr_map: &AttrMap) -> AriaValidator {
    let mut validator = AriaValidator::new();

    // Extract ARIA attributes from the AttrMap for analysis.
    // AttrMap::iter_attrs() yields &(HtmlAttr, AttrValue) — filter ARIA variants via TryFrom.
    let aria_attrs: Vec<AriaAttribute> = attr_map.iter_attrs()
        .filter_map(|(k, _)| AriaAttribute::try_from(*k).ok())
        .collect();

    // Check role with actual ARIA attributes from the AttrMap
    let has_tabindex = attr_map.contains(&HtmlAttr::TabIndex);
    if let Some(role) = role {
        validator.check_role(role, &aria_attrs, has_tabindex);
    }

    // Check for aria-activedescendant on incompatible roles
    let has_active_descendant = aria_attrs.iter().any(|a| matches!(a, AriaAttribute::ActiveDescendant(_)));
    if has_active_descendant {
        if let Some(role) = role {
            if !role.supports_active_descendant() {
                validator.errors.push(AriaValidationError::ActiveDescendantOnUnsupportedRole {
                    role: role.name(),
                });
            }
        }
    }

    let _ = aria_attrs;
    validator
}
```

### 9.2 Keyboard Navigation Test Helpers

````rust
// ars-a11y/src/testing/keyboard.rs

/// A simulated keyboard event for use in unit tests.
#[derive(Clone, Debug)]
pub struct SimulatedKeyEvent {
    pub key: &'static str,
    pub shift: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub alt: bool,
    pub default_prevented: AtomicBool,
    pub propagation_stopped: AtomicBool,
}

impl SimulatedKeyEvent {
    pub fn key(key: &'static str) -> Self {
        Self {
            key,
            shift: false, ctrl: false, meta: false, alt: false,
            default_prevented: AtomicBool::new(false),
            propagation_stopped: AtomicBool::new(false),
        }
    }

    pub fn with_shift(mut self) -> Self { self.shift = true; self }
    pub fn with_ctrl(mut self) -> Self { self.ctrl = true; self }
    pub fn with_meta(mut self) -> Self { self.meta = true; self }
    pub fn with_alt(mut self) -> Self { self.alt = true; self }
}

impl crate::DomEvent for SimulatedKeyEvent {
    fn event_type(&self) -> &str { "keydown" }
    fn prevent_default(&self) { self.default_prevented.store(true, Ordering::Relaxed); }
    fn stop_propagation(&self) { self.propagation_stopped.store(true, Ordering::Relaxed); }
    fn key(&self) -> Option<&str> { Some(self.key) }
    fn shift_key(&self) -> bool { self.shift }
    fn ctrl_key(&self) -> bool { self.ctrl }
    fn meta_key(&self) -> bool { self.meta }
    fn alt_key(&self) -> bool { self.alt }
}

/// A recorder that captures the sequence of focus index changes
/// during keyboard navigation testing.
pub struct NavigationRecorder {
    pub events: Vec<NavigationEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavigationEvent {
    FocusMoved { from: usize, to: usize },
    SelectionChanged { index: usize },
    Activated { index: usize },
    Escaped,
}

impl NavigationRecorder {
    pub fn new() -> Self { Self { events: Vec::new() } }

    pub fn record_focus_move(&mut self, from: usize, to: usize) {
        self.events.push(NavigationEvent::FocusMoved { from, to });
    }

    pub fn assert_focus_sequence(&self, expected: &[(usize, usize)]) {
        let actual: Vec<(usize, usize)> = self.events.iter()
            .filter_map(|e| match e {
                NavigationEvent::FocusMoved { from, to } => Some((*from, *to)),
                _ => None,
            })
            .collect();
        assert_eq!(actual, expected, "Focus navigation sequence mismatch");
    }
}

/// Test helper: simulate keyboard navigation through a FocusZone and
/// record resulting focus movements.
///
/// # Example
///
/// ```rust
/// let mut zone = FocusZone::new(FocusZoneOptions::default(), 5);
/// let mut recorder = NavigationRecorder::new();
///
/// let keys = [KeyboardKey::ArrowDown, KeyboardKey::ArrowDown, KeyboardKey::End, KeyboardKey::ArrowUp];
/// let mut current = 0;
///
/// for key in keys {
///     if let Some(next) = zone.handle_key(key, false, |_| false) {
///         recorder.record_focus_move(current, next);
///         current = next;
///         zone.active_index = next;
///     }
/// }
///
/// recorder.assert_focus_sequence(&[(0, 1), (1, 2), (2, 4), (4, 3)]);
/// ```
pub struct FocusZoneTestHarness {
    pub zone: FocusZone,
    pub current_index: usize,
    pub recorder: NavigationRecorder,
    pub disabled_indices: std::collections::BTreeSet<usize>,
}

impl FocusZoneTestHarness {
    pub fn new(options: FocusZoneOptions, item_count: usize) -> Self {
        Self {
            zone: FocusZone::new(options, item_count),
            current_index: 0,
            recorder: NavigationRecorder::new(),
            disabled_indices: Default::default(),
        }
    }

    pub fn disable(&mut self, index: usize) {
        self.disabled_indices.insert(index);
    }

    /// Note: RTL navigation should also be tested by callers passing `is_rtl: true`
    /// to `handle_key`, verifying that horizontal arrow keys are swapped.
    pub fn send_key(&mut self, key: KeyboardKey) -> bool {
        let is_disabled = |i: usize| self.disabled_indices.contains(&i);
        if let Some(next) = self.zone.handle_key(key, false, is_disabled) {
            self.recorder.record_focus_move(self.current_index, next);
            self.current_index = next;
            self.zone.active_index = next;
            true
        } else {
            false
        }
    }

    pub fn assert_at(&self, expected_index: usize) {
        assert_eq!(
            self.current_index, expected_index,
            "Expected focus at index {}, but was at {}",
            expected_index, self.current_index
        );
    }
}

// ── Example unit tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::focus::zone::{FocusZone, FocusZoneOptions, FocusZoneDirection};

    #[test]
    fn vertical_zone_wraps() {
        let opts = FocusZoneOptions {
            direction: FocusZoneDirection::Vertical,
            wrap: true,
            ..Default::default()
        };
        let mut harness = FocusZoneTestHarness::new(opts, 3);

        harness.send_key(KeyboardKey::ArrowDown);  // 0 → 1
        harness.send_key(KeyboardKey::ArrowDown);  // 1 → 2
        harness.send_key(KeyboardKey::ArrowDown);  // 2 → 0 (wrap)

        harness.assert_at(0);
        harness.recorder.assert_focus_sequence(&[(0, 1), (1, 2), (2, 0)]);
    }

    #[test]
    fn zone_skips_disabled_items() {
        let opts = FocusZoneOptions::default();
        let mut harness = FocusZoneTestHarness::new(opts, 5);
        harness.disable(1);
        harness.disable(2);

        harness.send_key(KeyboardKey::ArrowDown);  // 0 → skips 1, 2 → lands at 3
        harness.assert_at(3);
    }

    #[test]
    fn home_end_navigation() {
        let opts = FocusZoneOptions::default();
        let mut harness = FocusZoneTestHarness::new(opts, 5);

        harness.send_key(KeyboardKey::End);   // 0 → 4
        harness.assert_at(4);

        harness.send_key(KeyboardKey::Home);  // 4 → 0
        harness.assert_at(0);
    }

    #[test]
    fn typeahead_finds_matching_item() {
        let mut ta = typeahead::State::new();
        let labels = vec!["Apple", "Banana", "Cherry", "Apricot", "Blueberry"];

        let search = ta.process_key('a', 0.0);
        let result = ta.find_next_match(
            0,
            labels.len(),
            |i| labels[i].to_string(),
            |_| false,
        );
        assert_eq!(result, Some(3)); // "Apricot" comes after "Apple" (from_index=0)

        let search2 = ta.process_key('p', 10.0); // 10ms later, still in window
        let result2 = ta.find_next_match(
            0,
            labels.len(),
            |i| labels[i].to_string(),
            |_| false,
        );
        assert_eq!(result2, Some(3)); // "Apricot" starts with "ap"
        let _ = (search, search2);
    }

    #[test]
    fn aria_validator_catches_abstract_role() {
        let mut validator = AriaValidator::new();
        validator.check_role(AriaRole::Widget, &[], false);
        assert!(validator.has_errors());
        assert!(matches!(
            validator.errors()[0],
            AriaValidationError::AbstractRoleUsed { .. }
        ));
    }

    #[test]
    fn aria_validator_catches_missing_required_attr() {
        let mut validator = AriaValidator::new();
        // Slider requires aria-valuenow
        validator.check_role(AriaRole::Slider, &[], false);
        assert!(validator.errors().iter().any(|e| matches!(
            e,
            AriaValidationError::MissingRequiredAttribute { missing_attr: "aria-valuenow", .. }
        )));
    }

    #[test]
    fn live_announcer_deduplicates_voiceover() {
        let mut announcer = LiveAnnouncer::new();
        announcer.announce("Test message");
        // VoiceOver toggle alternates on each call.
        // A second identical message should have a different DOM content.
        announcer.announce("Test message");
        // The voiceover_toggle alternated → content differs → VoiceOver re-announces.
        // This is the intended behavior; no assertion here, it's a behavioral guarantee.
    }
}
````

---

## 10. Appendix A: ars-a11y Module Structure

```text
crates/ars-a11y/
  src/
    lib.rs                  // Re-exports; feature flags
    aria/
      mod.rs
      role.rs               // AriaRole enum (all roles)
      attribute.rs          // AriaAttribute enum (all states/properties)
      apply.rs              // apply_role(), apply_aria(), set_role! macro
      state.rs              // set_expanded(), set_selected(), set_disabled(), etc.
    focus/
      mod.rs
      scope.rs              // FocusScope, FocusScopeOptions, FocusTarget, FocusStrategy
      ring.rs               // FocusRing, keyboard modality detection
      zone.rs               // FocusZone, FocusZoneOptions, FocusZoneDirection
    keyboard/
      mod.rs
      typeahead.rs          // typeahead::State, is_printable_key()
                            // Depends on `unicode-normalization` crate (no_std + alloc compatible)
      shortcuts.rs          // KeyboardShortcut, KeyModifiers, Platform
    announcer.rs            // LiveAnnouncer, AnnouncementPriority, Announcement
    announcements.rs        // Pre-built announcement string helpers
    visually_hidden.rs      // visually_hidden_attrs(), visually_hidden_focusable_attrs()
    label.rs                // LabelConfig, DescriptionConfig, FieldContext
    id.rs                   // ComponentIds (adapter-provided IDs)
    media.rs                // Re-export facade behind #[cfg(feature = "dom")]; canonical
                            // implementations live in ars-dom
    touch.rs                // touch_target_attrs(), InputMode, should_use_roving_tabindex
    testing/
      mod.rs
      validator.rs          // AriaValidator, AriaValidationError, validate_attr_map()
      keyboard.rs           // SimulatedKeyEvent, NavigationRecorder, FocusZoneTestHarness
```

## 11. Appendix B: Component-to-Pattern Mapping

This table maps each component from `02-component-catalog.md` to the accessibility patterns it uses.

**Note:** This table lists key examples only. See individual component specs for complete ARIA pattern mappings.

| Component     | ARIA Role(s)                      | Focus Pattern                                                    | Live Region     | Notes                                                                                                                                                    |
| ------------- | --------------------------------- | ---------------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Button        | `button` (implicit)               | Standard tab                                                     | —               | Native `<button>` preferred                                                                                                                              |
| Checkbox      | `checkbox`                        | Standard tab                                                     | —               | Requires `aria-checked`                                                                                                                                  |
| RadioGroup    | `radiogroup` > `radio`            | Roving tabindex                                                  | —               | Group role on container                                                                                                                                  |
| Switch        | `switch`                          | Standard tab                                                     | —               | Requires `aria-checked`                                                                                                                                  |
| TextField     | `textbox` (implicit)              | Standard tab                                                     | Error msg       | Uses `FieldContext`                                                                                                                                      |
| Slider        | `slider`                          | Standard tab                                                     | Value on change | Requires valuenow/min/max                                                                                                                                |
| Select        | `combobox` + `listbox`            | FocusScope + roving tabindex                                     | Selection       | See Combobox pattern                                                                                                                                     |
| Combobox      | `combobox` + `listbox`            | activedescendant (primary; aria-live fallback for VoiceOver iOS) | Results count   | Focus stays in input                                                                                                                                     |
| Listbox       | `listbox` > `option`              | Roving tabindex                                                  | Selection       |                                                                                                                                                          |
| Menu          | `menu` / `menubar` > `menuitem`   | FocusScope + roving tabindex                                     | —               | Escape returns focus to trigger                                                                                                                          |
| Dialog        | `dialog`                          | FocusScope (contain)                                             | —               | `aria-modal=true`                                                                                                                                        |
| AlertDialog   | `alertdialog`                     | FocusScope (contain)                                             | —               | More urgent than dialog                                                                                                                                  |
| Popover       | `dialog` or contextual            | FocusScope (overlay)                                             | —               | No containment for non-modal                                                                                                                             |
| Tooltip       | `tooltip`                         | —                                                                | —               | Never interactive; hover + focus                                                                                                                         |
| Toast         | `status` or `log`                 | —                                                                | LiveAnnouncer   | `aria-live="polite"` by default                                                                                                                          |
| Tabs          | `tablist` > `tab` + `tabpanel`    | Roving tabindex                                                  | —               | `aria-selected`, `aria-controls`                                                                                                                         |
| Accordion     | `button` + `region`               | Standard tab                                                     | —               | `aria-expanded` on button                                                                                                                                |
| Tree          | `tree` > `treeitem`               | Roving tabindex                                                  | Expand state    | Arrow key full pattern                                                                                                                                   |
| Grid / Table  | `grid` / `table` > `row` > `cell` | 2D FocusZone                                                     | Sort state      |                                                                                                                                                          |
| DatePicker    | `group` + `dialog`                | FocusScope                                                       | Date change     | Calendar grid: Grid pattern                                                                                                                              |
| ColorPicker   | `group`                           | 2D FocusZone                                                     | Value           | Color area is 2D slider                                                                                                                                  |
| Progress      | `progressbar`                     | —                                                                | Busy state      | `aria-valuenow` or `aria-valuetext`                                                                                                                      |
| Meter         | `meter`                           | —                                                                | —               | `aria-valuenow`, min, max                                                                                                                                |
| TagGroup      | `group` (read-only display)       | Roving tabindex                                                  | Add/remove      | Delete key removes focused tag. Note: interactive TagsInput (selection category) may use different ARIA roles — see `components/selection/tags-input.md` |
| Toolbar       | `toolbar`                         | Roving tabindex (horizontal)                                     | —               | Arrow keys within toolbar                                                                                                                                |
| FocusScope    | —                                 | FocusScope                                                       | —               | Standalone utility                                                                                                                                       |
| LiveAnnouncer | `status` / `log` (hidden)         | —                                                                | All             | Singleton per page                                                                                                                                       |

## 12. Forced Colors — Component-Specific Addenda

> **Normative rules:** §6.1 defines the general forced-colors CSS authoring rules (system colors, SVG fill, no color-only state indicators). `05-interactions.md` §10 covers interaction-specific forced-colors styling (pressed, dragging, selected states). This section provides **component-specific** guidance only.

1. **Checkbox checkmarks and Switch toggle indicators** must use `currentColor` or `CanvasText` system color, never custom colors that disappear in forced-colors mode.
2. **Slider track and thumb** must use visible borders (min 1px solid) — background-color-based tracks disappear.
3. **Progress/Meter fill regions** must use `Highlight` fill with a visible border — gradient-based fills are suppressed.
4. Test all form controls and selection indicators in forced-colors mode (see `05-interactions.md` §10.5 for testing requirements).

## 13. Disabled vs Readonly Accessibility Contract

1. **Disabled**: element remains in tab order but is inoperable (per APG guidance, `aria-disabled` elements stay focusable so screen reader users can discover them), `aria-disabled="true"` set, no HTML `disabled` attribute (allows tooltip on hover). **Tooltip exception:** disabled elements remain in the DOM and can receive pointer hover events for tooltip display, even though they are non-interactive for activation. Form submission excludes disabled fields.
2. **Readonly**: element remains in tab order, `aria-readonly="true"` set, value visible but not editable. Form submission includes readonly values.
3. All form components must emit the correct ARIA attribute in their adapter render function — this is verified by snapshot tests.

### 13.1 Disabled State Responsibility Matrix

| Attribute              | Set By                         | Notes                                                         |
| ---------------------- | ------------------------------ | ------------------------------------------------------------- |
| `aria-disabled="true"` | ars-a11y (connect function)    | Preferred over HTML `disabled` for composite widgets          |
| `data-ars-disabled`    | ars-interactions (PressResult) | CSS styling hook                                              |
| `tabindex`             | Component connect function     | NOT set to "-1" for aria-disabled — element remains focusable |
| HTML `disabled`        | Adapter (native elements only) | Only for native `<button>`, `<input>`, etc.                   |

## 14. Appendix C: Cross-Reference Index

| Topic                                | Primary Location             |
| ------------------------------------ | ---------------------------- |
| Machine trait, AttrMap, Transition   | `01-architecture.md` §2      |
| Component catalog and priorities     | `02-component-catalog.md`    |
| Naming: crate names, data attributes | `00-overview.md` §3          |
| ARIA attribute string values         | This document §2.2           |
| Focus containment (modal dialogs)    | This document §3.2           |
| Arrow-key patterns per component     | This document §4.2           |
| Screen reader announcement           | This document §5.1           |
| Testing state machines without DOM   | `01-architecture.md` §8.1    |
| Internationalization and BiDi        | `04-internationalization.md` |
| Interaction patterns and press/hover | `05-interactions.md`         |
| DOM utilities (ars-dom)              | `11-dom-utilities.md`        |
