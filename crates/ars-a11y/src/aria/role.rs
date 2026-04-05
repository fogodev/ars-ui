//! WAI-ARIA 1.2 role system.
//!
//! Defines the [`AriaRole`](crate::AriaRole) enum covering all WAI-ARIA 1.2 role definitions
//! (plus select WAI-ARIA 1.3 draft roles), organised by category: abstract,
//! window, widget, composite, document structure, live region, and landmark.
//!
//! Reference: <https://www.w3.org/TR/wai-aria-1.2/#role_definitions>

/// A WAI-ARIA role that conveys the semantic purpose of a DOM element.
///
/// Roles are grouped into categories per WAI-ARIA 1.2. Abstract roles are
/// included for completeness but must never appear on DOM elements;
/// [`AriaRole::to_attr_value`] returns `None` for them.
///
/// Reference: <https://www.w3.org/TR/wai-aria-1.2/#role_definitions>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AriaRole {
    // -- Abstract roles (do not use on DOM elements) --------------------------
    // Included for completeness; `to_attr_value` returns None for these.
    /// Abstract superclass for interactive widgets that perform an action.
    Command,
    /// Abstract superclass for widgets that contain navigable descendants.
    Composite,
    /// Abstract superclass for widgets that accept user input.
    Input,
    /// Abstract superclass for perceivable page regions.
    Landmark,
    /// Abstract superclass for widgets representing a numeric value within a range.
    Range,
    /// Abstract superclass for the base role from which all other roles inherit.
    RoleType,
    /// Abstract superclass for rendering a structural containment area.
    Section,
    /// Abstract superclass for labelling or summarising a section topic.
    SectionHead,
    /// Abstract superclass for form widgets that allow a selection from options.
    Select,
    /// Abstract superclass for document structural elements.
    Structure,
    /// Abstract superclass for interactive UI components.
    Widget,
    /// Abstract superclass for window-like elements (dialog, alertdialog).
    Window,

    // -- Window roles ---------------------------------------------------------
    /// An alert dialog that interrupts workflow and demands acknowledgement.
    Alertdialog,
    /// A modal or non-modal dialog window.
    Dialog,

    // -- Widget roles ---------------------------------------------------------
    /// An interactive element that triggers an action when activated.
    Button,
    /// A checkable input with `true`, `false`, or `mixed` states.
    Checkbox,
    /// A cell in a grid or treegrid that may contain interactive content.
    Gridcell,
    /// An interactive reference to a resource.
    Link,
    /// An option in a menu that triggers an action.
    Menuitem,
    /// A checkable menuitem with `true` or `false` state.
    Menuitemcheckbox,
    /// A checkable menuitem within a group where only one may be checked.
    Menuitemradio,
    /// A selectable item within a listbox.
    ///
    /// **Warning:** `AriaRole::Option` shadows `core::option::Option` if you
    /// use `use AriaRole::*`. Prefer qualified access: `AriaRole::Option`.
    Option,
    /// An element displaying the progress of a long-running task.
    Progressbar,
    /// A checkable input within a group where only one may be checked.
    Radio,
    /// A graphical object controlling the scrolling of content within a viewport.
    Scrollbar,
    /// A textbox intended for search queries.
    Searchbox,
    /// Focusable separator (widget role) with value semantics.
    ///
    /// Validators: a focusable `Separator` element has `tabindex >= 0` and
    /// requires value attributes (`aria-valuenow`, `aria-valuemin`,
    /// `aria-valuemax`).
    Separator,
    /// A user-input element constraining its value to a range.
    Slider,
    /// A range whose value can be discreetly adjusted by the user.
    Spinbutton,
    /// An input representing on/off values.
    Switch,
    /// An interactive element inside a tablist that controls a tabpanel.
    Tab,
    /// A container for the content associated with a tab.
    Tabpanel,
    /// An element that accepts free-form text input.
    Textbox,
    /// A selectable item within a tree.
    Treeitem,

    // -- Composite widget roles -----------------------------------------------
    /// A composite widget combining a text input with a popup providing values.
    Combobox,
    /// A composite widget containing a collection of rows with cells.
    Grid,
    /// A widget presenting a list of selectable options.
    Listbox,
    /// A widget offering a list of actions or functions the user can invoke.
    Menu,
    /// A menu that is visually persistent, typically horizontal.
    Menubar,
    /// A group of radio buttons where only one may be selected at a time.
    Radiogroup,
    /// A list of tab elements controlling the display of tabpanels.
    Tablist,
    /// A widget presenting a hierarchical list of selectable items.
    Tree,
    /// A grid whose rows can be expanded/collapsed like a tree.
    Treegrid,

    // -- Document structure roles ---------------------------------------------
    /// A region declared as a web application rather than a web document.
    Application,
    /// An independent section of a page forming a self-contained composition.
    Article,
    /// A section of quoted content from another source.
    BlockQuote,
    /// A visible label or caption for another object.
    Caption,
    /// A cell in a table or grid.
    Cell,
    /// An inline section of computer code.
    Code,
    /// A header cell in a grid or table containing header information for a column.
    Columnheader,
    /// A comment or annotation on content.
    ///
    /// WAI-ARIA 1.3 draft -- intentionally included; see module-level note.
    Comment,
    /// A definition of a term or concept.
    Definition,
    /// Content that has been removed or flagged for removal.
    Deletion,
    /// A list of references to members of a group.
    ///
    /// Superseded by `list` in ARIA 1.2; retained for assistive technology
    /// interoperability.
    Directory,
    /// An element containing content oriented for reading, not interaction.
    Document,
    /// Inline content that has stress emphasis.
    Emphasis,
    /// A scrollable list of articles where new articles may be added dynamically.
    Feed,
    /// A perceivable section with optional caption, used for images, code, etc.
    Figure,
    /// A nameless container with no semantic meaning.
    Generic,
    /// A set of related UI elements not intended to be included in a page summary.
    Group,
    /// A heading for a section of the page.
    Heading,
    /// An image element.
    Img,
    /// Content that has been added or flagged for addition.
    Insertion,
    /// An ordered or unordered collection of items.
    List,
    /// A single item within a list.
    Listitem,
    /// Content highlighted for reference or notation purposes.
    Mark,
    /// Content representing a mathematical expression.
    Math,
    /// A scalar measurement within a known range.
    Meter,
    /// Removes the element's implicit ARIA semantics from the accessibility tree.
    ///
    /// Same semantic as [`AriaRole::Presentation`].
    None,
    /// An advisory section whose content is parenthetic or ancillary.
    Note,
    /// A paragraph of content.
    Paragraph,
    /// Removes the element's implicit ARIA semantics from the accessibility tree.
    ///
    /// Same semantic as [`AriaRole::None`].
    Presentation,
    /// A row within a grid, table, or treegrid.
    Row,
    /// A group of rows within a grid, table, or treegrid.
    Rowgroup,
    /// A header cell containing header information for a row.
    Rowheader,
    /// Inline text with strong importance.
    Strong,
    /// Subscript text.
    Subscript,
    /// A revision or annotation proposed by an author or reviewer.
    ///
    /// WAI-ARIA 1.3 draft -- intentionally included; see module-level note.
    Suggestion,
    /// Superscript text.
    Superscript,
    /// A data arrangement in rows and columns (non-interactive).
    Table,
    /// A word or phrase with an optional corresponding definition.
    Term,
    /// An element representing a specific point in time.
    Time,
    /// A collection of commonly used function buttons or controls.
    Toolbar,
    /// A contextual popup that displays a description for an element.
    Tooltip,

    // -- Live region roles ----------------------------------------------------
    /// An important, usually time-sensitive, live-region message.
    Alert,
    /// A live region where new information is added in meaningful order.
    Log,
    /// A non-essential live region whose content changes automatically.
    Marquee,
    /// A live region containing advisory information.
    Status,
    /// A live region containing a numerical counter indicating elapsed time.
    Timer,

    // -- Landmark roles -------------------------------------------------------
    /// Site-oriented content, typically including the site logo and heading.
    Banner,
    /// A supporting section of the document complementing the main content.
    Complementary,
    /// Information about the parent document (footer).
    Contentinfo,
    /// A landmark region containing a collection of form-associated elements.
    Form,
    /// The main content of the document.
    Main,
    /// A collection of navigational elements for the document or related documents.
    Navigation,
    /// A perceivable section containing content relevant to a specific purpose.
    Region,
    /// A landmark containing a search facility.
    Search,

    /// Non-focusable separator (document structure role).
    ///
    /// Validators: distinguish from [`AriaRole::Separator`] by absence of tabindex.
    /// `StructuralSeparator` elements have no tabindex; focusable `Separator`
    /// elements have `tabindex >= 0` and require value attributes.
    StructuralSeparator,
}

// WAI-ARIA 1.3 draft roles: `Comment` and `Suggestion` are included
// intentionally. ars-ui tracks the WAI-ARIA specification as it evolves.
// These roles have broad assistive technology support already (NVDA 2024+,
// JAWS 2024+) and are on track for inclusion in the final 1.3 recommendation.
// If a role is removed from the draft before finalization, we will remove it
// here and update all references.

impl AriaRole {
    /// Returns the WAI-ARIA role string, or `None` for abstract roles.
    #[must_use]
    pub const fn to_attr_value(self) -> Option<&'static str> {
        match self {
            // Abstract roles -- MUST NOT appear in DOM
            Self::Command
            | Self::Composite
            | Self::Input
            | Self::Landmark
            | Self::Range
            | Self::RoleType
            | Self::Section
            | Self::SectionHead
            | Self::Select
            | Self::Structure
            | Self::Widget
            | Self::Window => None,

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
            Self::Comment => Some("comment"),
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
            Self::Suggestion => Some("suggestion"),
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

            // Separator is overloaded: widget (focusable, with value semantics)
            // vs structural. Both map to "separator" in the DOM. AriaValidator
            // should warn if StructuralSeparator is used on an element with
            // tabindex >= 0, or if Separator (widget) lacks tabindex and value
            // attributes.
            Self::Separator | Self::StructuralSeparator => Some("separator"),
        }
    }

    /// Returns `true` if this role supports `aria-activedescendant`.
    #[must_use]
    pub const fn supports_active_descendant(self) -> bool {
        matches!(
            self,
            Self::Application
                | Self::Combobox
                | Self::Grid
                | Self::Group
                | Self::Listbox
                | Self::Menu
                | Self::Menubar
                | Self::Radiogroup
                // Row supports aria-activedescendant only within grid/treegrid context
                | Self::Row
                | Self::Spinbutton
                | Self::Tablist
                // Note: Searchbox is intentionally excluded per WAI-ARIA 1.2.
                // While searchbox inherits from textbox (which supports
                // aria-activedescendant), the WAI-ARIA spec does not explicitly
                // list searchbox as supporting it. Autocomplete/combobox patterns
                // should use the Combobox role instead.
                | Self::Textbox
                | Self::Toolbar
                | Self::Tree
                | Self::Treegrid
        )
    }

    /// Returns `true` if this is an abstract role that should never be used on
    /// DOM elements.
    #[must_use]
    pub const fn is_abstract(self) -> bool {
        self.to_attr_value().is_none()
    }

    /// Returns the ARIA string value for concrete roles (e.g., `"button"`),
    /// or the Rust variant name for abstract roles (e.g., `"Widget"`).
    #[must_use]
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
            },
        }
    }

    /// Returns the required owned elements for this role.
    ///
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
    #[must_use]
    pub fn required_owned_elements(self) -> &'static [&'static [AriaRole]] {
        match self {
            Self::Feed => &[&[Self::Article]],
            Self::Grid | Self::Table | Self::Treegrid => &[&[Self::Row], &[Self::Rowgroup]],
            Self::List => &[&[Self::Listitem]],
            Self::Listbox => &[&[Self::Option], &[Self::Group]],
            Self::Menu | Self::Menubar => &[
                &[Self::Menuitem],
                &[Self::Menuitemcheckbox],
                &[Self::Menuitemradio],
                &[Self::Group],
            ],
            Self::Radiogroup => &[&[Self::Radio]],
            Self::Row => &[
                &[Self::Cell],
                &[Self::Columnheader],
                &[Self::Gridcell],
                &[Self::Rowheader],
            ],
            Self::Rowgroup => &[&[Self::Row]],
            Self::Tablist => &[&[Self::Tab]],
            Self::Tree => &[&[Self::Treeitem], &[Self::Group]],
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abstract_role_returns_none() {
        assert!(AriaRole::Command.to_attr_value().is_none());
    }

    #[test]
    fn concrete_role_returns_value() {
        assert_eq!(AriaRole::Button.to_attr_value().unwrap(), "button");
    }

    #[test]
    fn separator_role_is_overloaded() {
        assert_eq!(AriaRole::Separator.to_attr_value().unwrap(), "separator");
        assert_eq!(
            AriaRole::StructuralSeparator.to_attr_value().unwrap(),
            "separator"
        );
    }

    #[test]
    fn supports_active_descendant_positive() {
        assert!(AriaRole::Combobox.supports_active_descendant());
    }

    #[test]
    fn supports_active_descendant_negative() {
        assert!(!AriaRole::Button.supports_active_descendant());
    }

    #[test]
    fn required_owned_elements_for_list() {
        let required = AriaRole::List.required_owned_elements();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], &[AriaRole::Listitem]);
    }
}
