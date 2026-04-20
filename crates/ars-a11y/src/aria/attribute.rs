use alloc::{string::String, vec::Vec};
use core::fmt::{self, Display};

use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};

// ── Supporting types ──────────────────────────────────────────────────────────

/// A single ARIA ID reference (used in `aria-activedescendant`, `aria-details`, etc.).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AriaIdRef(pub String);

/// A space-separated list of ARIA ID references.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AriaIdList(pub Vec<String>);

impl AriaIdList {
    /// Creates a new, empty ID list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an ID to this list.
    pub fn push(&mut self, id: impl Into<String>) {
        self.0.push(id.into());
    }

    /// Returns `true` if the list contains no IDs.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for AriaIdList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;

        for id in &self.0 {
            if first {
                first = false;
            } else {
                f.write_str(" ")?;
            }

            f.write_str(id)?;
        }

        Ok(())
    }
}

/// The `aria-autocomplete` property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaAutocomplete {
    /// No autocompletion.
    None,

    /// Inline completion suggestion after the caret.
    Inline,

    /// A list of completion values is presented.
    List,

    /// Both inline and list completion.
    Both,
}

impl AriaAutocomplete {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Inline => "inline",
            Self::List => "list",
            Self::Both => "both",
        }
    }
}

/// The `aria-current` state values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaCurrent {
    /// Not the current item.
    False,

    /// The current item (generic).
    True,

    /// The current page within a set of pages.
    Page,

    /// The current step within a process.
    Step,

    /// The current location within an environment or context.
    Location,

    /// The current date within a date range.
    Date,

    /// The current time within a time range.
    Time,
}

impl AriaCurrent {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
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

/// The `aria-haspopup` property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaHasPopup {
    /// No popup.
    False,

    /// Has a popup (generic).
    True,

    /// Has a menu popup.
    Menu,

    /// Has a listbox popup.
    Listbox,

    /// Has a tree popup.
    Tree,

    /// Has a grid popup.
    Grid,

    /// Has a dialog popup.
    Dialog,
}

impl AriaHasPopup {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
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

/// The `aria-invalid` state values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AriaInvalid {
    /// The value is not invalid.
    False,

    /// The value is invalid.
    True,

    /// A grammatical error was detected.
    Grammar,

    /// A spelling error was detected.
    Spelling,
}

impl AriaInvalid {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Grammar => "grammar",
            Self::Spelling => "spelling",
        }
    }
}

/// The `aria-live` property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaLive {
    /// Updates will not be announced.
    Off,

    /// Updates will be announced at the next graceful opportunity.
    Polite,

    /// Updates will be announced immediately.
    Assertive,
}

impl AriaLive {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Polite => "polite",
            Self::Assertive => "assertive",
        }
    }
}

/// The `aria-orientation` property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaOrientation {
    /// The element is oriented horizontally.
    Horizontal,

    /// The element is oriented vertically.
    Vertical,

    /// The orientation is unknown or ambiguous.
    Undefined,
}

impl AriaOrientation {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Undefined => "undefined",
        }
    }
}

/// The `aria-pressed` state values for toggle buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaPressed {
    /// The button is not pressed.
    False,

    /// The button is pressed.
    True,

    /// The button is in a mixed pressed state.
    Mixed,
}

impl AriaPressed {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Mixed => "mixed",
        }
    }
}

/// The `aria-checked` state values for checkboxes and radio buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaChecked {
    /// The element is not checked.
    False,

    /// The element is checked.
    True,

    /// The element is in a mixed (indeterminate) checked state.
    Mixed,
}

impl AriaChecked {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Mixed => "mixed",
        }
    }
}

/// The `aria-sort` property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaSort {
    /// Items are not sorted.
    None,

    /// Items are sorted in ascending order.
    Ascending,

    /// Items are sorted in descending order.
    Descending,

    /// Items are sorted in an order other than ascending or descending.
    Other,
}

impl AriaSort {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Ascending => "ascending",
            Self::Descending => "descending",
            Self::Other => "other",
        }
    }
}

/// The `aria-relevant` property, indicating which mutations to a live region
/// are relevant for assistive technology announcements.
#[derive(Clone, Debug, PartialEq)]
pub struct AriaRelevant {
    /// Node additions are relevant.
    pub additions: bool,

    /// Node removals are relevant.
    pub removals: bool,

    /// Text content changes are relevant.
    pub text: bool,
}

impl Display for AriaRelevant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if self.additions {
            parts.push("additions");
        }

        if self.removals {
            parts.push("removals");
        }

        if self.text {
            parts.push("text");
        }

        if parts.is_empty() {
            // All-false returns empty string so the attribute is omitted, letting
            // the browser apply its default (`additions text`).
            return write!(f, "");
        }

        let mut first = true;

        for part in &parts {
            if first {
                first = false;
            } else {
                f.write_str(" ")?;
            }

            f.write_str(part)?;
        }

        Ok(())
    }
}

impl Default for AriaRelevant {
    fn default() -> Self {
        Self {
            additions: true,
            removals: false,
            text: true,
        }
    }
}

/// The WAI-ARIA 1.2 deprecated `aria-dropeffect` values.
///
/// Prefer `aria-description` for drop state communication.
/// Retained without `#[deprecated]` per project no-deprecation policy;
/// gated behind `#[cfg(feature = "aria-drag-drop-compat")]`.
#[cfg(feature = "aria-drag-drop-compat")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaDropeffect {
    /// No drop effect.
    None,

    /// A copy of the source will be placed at the target.
    Copy,

    /// A function will be executed using the drag source.
    Execute,

    /// A reference to the source will be created at the target.
    Link,

    /// The source will be moved to the target.
    Move,

    /// A popup menu or dialog is presented for user selection.
    Popup,
}

#[cfg(feature = "aria-drag-drop-compat")]
impl AriaDropeffect {
    /// Returns the WAI-ARIA token for this value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Copy => "copy",
            Self::Execute => "execute",
            Self::Link => "link",
            Self::Move => "move",
            Self::Popup => "popup",
        }
    }
}

// ── Main enum ─────────────────────────────────────────────────────────────────

/// A typed WAI-ARIA attribute with its associated value.
///
/// This enum covers all non-deprecated ARIA 1.2 states and properties.
/// WAI-ARIA 1.2 deprecated attributes (`aria-grabbed`, `aria-dropeffect`) are
/// available behind `#[cfg(feature = "aria-drag-drop-compat")]` and should not
/// be used in new components. Use `aria-description` for drop state feedback instead.
///
/// Reference: <https://www.w3.org/TR/wai-aria-1.2/#state_prop_def>
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
    /// Preferred over `aria-dropeffect` for drag-and-drop descriptions.
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
    /// `None` removes the attribute (undefined -- element is not a toggle).
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
    /// -1 indicates the total count is unknown.
    SetSize(i32),

    /// Indicates if items in a table or grid are sorted in ascending, descending,
    /// or other order.
    Sort(AriaSort),

    /// Defines the maximum allowed value for a range widget.
    ValueMax(f64),

    /// Defines the minimum allowed value for a range widget.
    ValueMin(f64),

    /// Defines the current value for a range widget.
    ValueNow(f64),

    /// Defines the human-readable text alternative of `aria-valuenow` for a range widget.
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
    /// and other widgets. See also [`AriaPressed`].
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
    /// Serializes this attribute to its DOM string value, or `None` if the
    /// attribute should be removed (e.g., `aria-hidden=None`).
    #[must_use]
    pub fn to_attr_value(&self) -> Option<String> {
        use alloc::string::ToString;
        match self {
            Self::ActiveDescendant(id) => id.as_ref().map(|id| id.0.clone()),

            Self::AutoComplete(v) => Some(v.as_str().into()),

            Self::Controls(ids) => Some(ids.to_string()),

            Self::Current(v) => Some(v.as_str().into()),

            Self::DescribedBy(ids) => {
                let s = ids.to_string();
                if s.is_empty() { None } else { Some(s) }
            }

            Self::Description(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }

            Self::Details(id) => Some(id.0.clone()),

            Self::Disabled(v) => Some(v.to_string()),

            Self::FlowTo(ids) => Some(ids.to_string()),

            Self::HasPopup(v) => Some(v.as_str().into()),

            Self::Hidden(None) => None,

            Self::Hidden(Some(v)) => Some(v.to_string()),

            Self::Invalid(v) => Some(v.as_str().into()),

            Self::Label(s) => Some(s.clone()),

            Self::LabelledBy(ids) => {
                let s = ids.to_string();
                if s.is_empty() { None } else { Some(s) }
            }

            Self::Level(n) => Some(n.to_string()),

            Self::Modal(v) => Some(v.to_string()),

            Self::MultiLine(v) => Some(v.to_string()),

            Self::MultiSelectable(v) => Some(v.to_string()),

            Self::Orientation(v) => Some(v.as_str().into()),

            Self::Owns(ids) => Some(ids.to_string()),

            Self::Placeholder(s) => Some(s.clone()),

            Self::PosInSet(n) => Some(n.to_string()),

            Self::Pressed(None) => None,

            Self::Pressed(Some(v)) => Some(v.as_str().into()),

            Self::ReadOnly(v) => Some(v.to_string()),

            Self::Required(v) => Some(v.to_string()),

            Self::RoleDescription(s) => Some(s.clone()),

            Self::Selected(None) => None,

            Self::Selected(Some(v)) => Some(v.to_string()),

            Self::SetSize(n) => Some(n.to_string()),

            Self::Sort(v) => Some(v.as_str().into()),

            Self::ValueMax(n) => Some(n.to_string()),

            Self::ValueMin(n) => Some(n.to_string()),

            Self::ValueNow(n) => Some(n.to_string()),

            Self::ValueText(s) => Some(s.clone()),

            Self::Atomic(v) => Some(v.to_string()),

            Self::Busy(v) => Some(v.to_string()),

            Self::Live(v) => Some(v.as_str().into()),

            Self::Relevant(v) => Some(v.to_string()),

            #[cfg(feature = "aria-drag-drop-compat")]
            Self::DropEffect(v) => Some(v.as_str().into()),

            Self::ErrorMessage(id) => Some(id.0.clone()),

            Self::Checked(v) => Some(v.as_str().into()),

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

    /// Returns the HTML attribute name for this ARIA attribute.
    ///
    /// Delegates to [`AriaAttr::as_str()`] via the `From<&AriaAttribute>` conversion,
    /// keeping attribute name strings in a single source of truth.
    #[must_use]
    pub fn attr_name(&self) -> &'static str {
        AriaAttr::from(self).as_str()
    }

    /// Returns the [`HtmlAttr`] key for this ARIA attribute.
    #[must_use]
    pub fn to_html_attr(&self) -> HtmlAttr {
        HtmlAttr::Aria(AriaAttr::from(self))
    }

    /// Apply this attribute to an [`AttrMap`].
    ///
    /// String-valued attributes are set directly. Nullable attributes whose
    /// value is absent (e.g., `Hidden(None)`, `Pressed(None)`) are written as
    /// [`AttrValue::None`] so the adapter knows to remove the attribute from the DOM.
    pub fn apply_to(&self, attrs: &mut AttrMap) {
        let key = self.to_html_attr();

        if let Some(value) = self.to_attr_value() {
            attrs.set(key, value);
        } else {
            attrs.set(key, AttrValue::None);
        }
    }
}

// ── Bridging impls: AriaAttr ↔ AriaAttribute ─────────────────────────────────

/// Converts a discriminant key ([`AriaAttr`]) to an [`AriaAttribute`] with
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
            // All current AriaAttr variants are covered above. This arm is
            // required because AriaAttr is #[non_exhaustive]. If reached, a
            // new variant was added to ars-core without updating ars-a11y.
            _ => unreachable!("unknown AriaAttr variant — update ars-a11y to match"),
        }
    }
}

/// Extracts the [`AriaAttr`] discriminant from an [`HtmlAttr::Aria`] variant.
/// Returns `Err(original)` if the [`HtmlAttr`] is not an ARIA variant.
impl TryFrom<HtmlAttr> for AriaAttribute {
    type Error = HtmlAttr;

    fn try_from(attr: HtmlAttr) -> Result<Self, Self::Error> {
        match attr {
            HtmlAttr::Aria(a) => Ok(AriaAttribute::from(a)),
            other => Err(other),
        }
    }
}

/// Maps a data-carrying [`AriaAttribute`] back to its discriminant key.
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

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use super::*;

    #[test]
    fn aria_id_list_display() {
        let mut list = AriaIdList::new();

        list.push("a");
        list.push("b");

        assert_eq!(list.to_string(), "a b");
    }

    #[test]
    fn aria_id_list_new_starts_empty() {
        let list = AriaIdList::new();

        assert!(list.is_empty());
        assert_eq!(list.to_string(), "");
    }

    #[test]
    fn aria_enum_string_tokens_match_spec() {
        let autocomplete = [
            (AriaAutocomplete::None, "none"),
            (AriaAutocomplete::Inline, "inline"),
            (AriaAutocomplete::List, "list"),
            (AriaAutocomplete::Both, "both"),
        ];

        for (value, expected) in autocomplete {
            assert_eq!(value.as_str(), expected);
        }

        let current = [
            (AriaCurrent::False, "false"),
            (AriaCurrent::True, "true"),
            (AriaCurrent::Page, "page"),
            (AriaCurrent::Step, "step"),
            (AriaCurrent::Location, "location"),
            (AriaCurrent::Date, "date"),
            (AriaCurrent::Time, "time"),
        ];

        for (value, expected) in current {
            assert_eq!(value.as_str(), expected);
        }

        let has_popup = [
            (AriaHasPopup::False, "false"),
            (AriaHasPopup::True, "true"),
            (AriaHasPopup::Menu, "menu"),
            (AriaHasPopup::Listbox, "listbox"),
            (AriaHasPopup::Tree, "tree"),
            (AriaHasPopup::Grid, "grid"),
            (AriaHasPopup::Dialog, "dialog"),
        ];

        for (value, expected) in has_popup {
            assert_eq!(value.as_str(), expected);
        }

        let invalid = [
            (AriaInvalid::False, "false"),
            (AriaInvalid::True, "true"),
            (AriaInvalid::Grammar, "grammar"),
            (AriaInvalid::Spelling, "spelling"),
        ];

        for (value, expected) in invalid {
            assert_eq!(value.as_str(), expected);
        }

        let live = [
            (AriaLive::Off, "off"),
            (AriaLive::Polite, "polite"),
            (AriaLive::Assertive, "assertive"),
        ];

        for (value, expected) in live {
            assert_eq!(value.as_str(), expected);
        }

        let orientation = [
            (AriaOrientation::Horizontal, "horizontal"),
            (AriaOrientation::Vertical, "vertical"),
            (AriaOrientation::Undefined, "undefined"),
        ];

        for (value, expected) in orientation {
            assert_eq!(value.as_str(), expected);
        }

        let pressed = [
            (AriaPressed::False, "false"),
            (AriaPressed::True, "true"),
            (AriaPressed::Mixed, "mixed"),
        ];

        for (value, expected) in pressed {
            assert_eq!(value.as_str(), expected);
        }

        let checked = [
            (AriaChecked::False, "false"),
            (AriaChecked::True, "true"),
            (AriaChecked::Mixed, "mixed"),
        ];

        for (value, expected) in checked {
            assert_eq!(value.as_str(), expected);
        }

        let sort = [
            (AriaSort::None, "none"),
            (AriaSort::Ascending, "ascending"),
            (AriaSort::Descending, "descending"),
            (AriaSort::Other, "other"),
        ];

        for (value, expected) in sort {
            assert_eq!(value.as_str(), expected);
        }
    }

    #[cfg(feature = "aria-drag-drop-compat")]
    #[test]
    fn aria_dropeffect_string_tokens_match_spec() {
        let dropeffect = [
            (AriaDropeffect::None, "none"),
            (AriaDropeffect::Copy, "copy"),
            (AriaDropeffect::Execute, "execute"),
            (AriaDropeffect::Link, "link"),
            (AriaDropeffect::Move, "move"),
            (AriaDropeffect::Popup, "popup"),
        ];

        for (value, expected) in dropeffect {
            assert_eq!(value.as_str(), expected);
        }
    }

    #[test]
    fn aria_attribute_disabled_to_attr_value() {
        let attr = AriaAttribute::Disabled(true);

        assert_eq!(attr.to_attr_value(), Some("true".into()));
    }

    #[test]
    fn aria_attribute_hidden_none_removes() {
        let attr = AriaAttribute::Hidden(None);

        assert_eq!(attr.to_attr_value(), None);
    }

    #[test]
    fn aria_attribute_pressed_mixed() {
        let attr = AriaAttribute::Pressed(Some(AriaPressed::Mixed));

        assert_eq!(attr.to_attr_value(), Some("mixed".into()));
    }

    #[test]
    fn aria_relevant_default() {
        let relevant = AriaRelevant::default();

        assert_eq!(relevant.to_string(), "additions text");
    }

    #[test]
    fn aria_relevant_serializes_all_combinations() {
        assert_eq!(
            AriaRelevant {
                additions: false,
                removals: false,
                text: false,
            }
            .to_string(),
            ""
        );
        assert_eq!(
            AriaRelevant {
                additions: true,
                removals: false,
                text: false,
            }
            .to_string(),
            "additions"
        );
        assert_eq!(
            AriaRelevant {
                additions: false,
                removals: true,
                text: false,
            }
            .to_string(),
            "removals"
        );
        assert_eq!(
            AriaRelevant {
                additions: false,
                removals: false,
                text: true,
            }
            .to_string(),
            "text"
        );
        assert_eq!(
            AriaRelevant {
                additions: true,
                removals: true,
                text: true,
            }
            .to_string(),
            "additions removals text"
        );
    }

    #[test]
    fn to_attr_value_serializes_representative_attributes() {
        let mut ids = AriaIdList::new();

        ids.push("item-1");
        ids.push("item-2");

        let cases = [
            (
                AriaAttribute::ActiveDescendant(Some(AriaIdRef("active".into()))),
                Some("active"),
            ),
            (
                AriaAttribute::AutoComplete(AriaAutocomplete::Both),
                Some("both"),
            ),
            (AriaAttribute::Controls(ids.clone()), Some("item-1 item-2")),
            (AriaAttribute::Current(AriaCurrent::Step), Some("step")),
            (
                AriaAttribute::DescribedBy(ids.clone()),
                Some("item-1 item-2"),
            ),
            (
                AriaAttribute::Description("described".into()),
                Some("described"),
            ),
            (
                AriaAttribute::Details(AriaIdRef("details".into())),
                Some("details"),
            ),
            (AriaAttribute::Disabled(true), Some("true")),
            (AriaAttribute::FlowTo(ids.clone()), Some("item-1 item-2")),
            (AriaAttribute::HasPopup(AriaHasPopup::Grid), Some("grid")),
            (AriaAttribute::Hidden(Some(false)), Some("false")),
            (
                AriaAttribute::Invalid(AriaInvalid::Spelling),
                Some("spelling"),
            ),
            (AriaAttribute::Label("label".into()), Some("label")),
            (
                AriaAttribute::LabelledBy(ids.clone()),
                Some("item-1 item-2"),
            ),
            (AriaAttribute::Level(3), Some("3")),
            (AriaAttribute::Modal(true), Some("true")),
            (AriaAttribute::MultiLine(true), Some("true")),
            (AriaAttribute::MultiSelectable(true), Some("true")),
            (
                AriaAttribute::Orientation(AriaOrientation::Undefined),
                Some("undefined"),
            ),
            (AriaAttribute::Owns(ids.clone()), Some("item-1 item-2")),
            (
                AriaAttribute::Placeholder("placeholder".into()),
                Some("placeholder"),
            ),
            (AriaAttribute::PosInSet(4), Some("4")),
            (
                AriaAttribute::Pressed(Some(AriaPressed::True)),
                Some("true"),
            ),
            (AriaAttribute::ReadOnly(true), Some("true")),
            (AriaAttribute::Required(true), Some("true")),
            (AriaAttribute::RoleDescription("chip".into()), Some("chip")),
            (AriaAttribute::Selected(Some(true)), Some("true")),
            (AriaAttribute::SetSize(-1), Some("-1")),
            (
                AriaAttribute::Sort(AriaSort::Descending),
                Some("descending"),
            ),
            (AriaAttribute::ValueMax(9.5), Some("9.5")),
            (AriaAttribute::ValueMin(1.5), Some("1.5")),
            (AriaAttribute::ValueNow(4.5), Some("4.5")),
            (
                AriaAttribute::ValueText("four and a half".into()),
                Some("four and a half"),
            ),
            (AriaAttribute::Atomic(true), Some("true")),
            (AriaAttribute::Busy(true), Some("true")),
            (AriaAttribute::Live(AriaLive::Assertive), Some("assertive")),
            (
                AriaAttribute::Relevant(AriaRelevant {
                    additions: false,
                    removals: true,
                    text: true,
                }),
                Some("removals text"),
            ),
            (
                AriaAttribute::ErrorMessage(AriaIdRef("error".into())),
                Some("error"),
            ),
            (AriaAttribute::Checked(AriaChecked::True), Some("true")),
            (AriaAttribute::Expanded(Some(true)), Some("true")),
            (AriaAttribute::ColCount(-1), Some("-1")),
            (AriaAttribute::ColIndex(2), Some("2")),
            (AriaAttribute::ColSpan(3), Some("3")),
            (AriaAttribute::RowCount(-1), Some("-1")),
            (AriaAttribute::RowIndex(5), Some("5")),
            (AriaAttribute::RowSpan(6), Some("6")),
            (AriaAttribute::KeyShortcuts("Ctrl+K".into()), Some("Ctrl+K")),
            (AriaAttribute::ActiveDescendant(None), None),
            (AriaAttribute::Description(String::new()), None),
            (AriaAttribute::DescribedBy(AriaIdList::new()), None),
            (AriaAttribute::LabelledBy(AriaIdList::new()), None),
            (AriaAttribute::Hidden(None), None),
            (AriaAttribute::Pressed(None), None),
            (AriaAttribute::Selected(None), None),
            (AriaAttribute::Expanded(None), None),
        ];

        for (attr, expected) in cases {
            assert_eq!(attr.to_attr_value().as_deref(), expected, "{attr:?}");
        }
    }

    #[cfg(feature = "aria-drag-drop-compat")]
    #[test]
    fn compat_attributes_serialize_and_round_trip() {
        let drop_effect = AriaAttribute::DropEffect(AriaDropeffect::Popup);

        assert_eq!(drop_effect.to_attr_value().as_deref(), Some("popup"));
        assert_eq!(
            drop_effect.to_html_attr(),
            HtmlAttr::Aria(AriaAttr::DropEffect)
        );
        assert_eq!(AriaAttr::from(&drop_effect), AriaAttr::DropEffect);

        let grabbed_true = AriaAttribute::Grabbed(Some(true));

        assert_eq!(grabbed_true.to_attr_value().as_deref(), Some("true"));
        assert_eq!(
            grabbed_true.to_html_attr(),
            HtmlAttr::Aria(AriaAttr::Grabbed)
        );
        assert_eq!(AriaAttr::from(&grabbed_true), AriaAttr::Grabbed);

        let grabbed_none = AriaAttribute::Grabbed(None);

        assert_eq!(grabbed_none.to_attr_value(), None);

        let mut attrs = AttrMap::new();

        grabbed_none.apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Grabbed)));
    }

    // ── Bridge tests ─────────────────────────────────────────────────────────

    #[test]
    fn attr_name_returns_html_attribute_string() {
        assert_eq!(AriaAttribute::Disabled(true).attr_name(), "aria-disabled");
        assert_eq!(
            AriaAttribute::ActiveDescendant(None).attr_name(),
            "aria-activedescendant"
        );
        assert_eq!(
            AriaAttribute::LabelledBy(AriaIdList::new()).attr_name(),
            "aria-labelledby"
        );
        assert_eq!(AriaAttribute::ValueNow(0.5).attr_name(), "aria-valuenow");
        assert_eq!(
            AriaAttribute::KeyShortcuts(String::new()).attr_name(),
            "aria-keyshortcuts"
        );
    }

    #[test]
    fn to_html_attr_wraps_in_aria_variant() {
        use ars_core::{AriaAttr, HtmlAttr};

        assert_eq!(
            AriaAttribute::Checked(AriaChecked::True).to_html_attr(),
            HtmlAttr::Aria(AriaAttr::Checked),
        );
        assert_eq!(
            AriaAttribute::Expanded(Some(true)).to_html_attr(),
            HtmlAttr::Aria(AriaAttr::Expanded),
        );
        assert_eq!(
            AriaAttribute::ColCount(-1).to_html_attr(),
            HtmlAttr::Aria(AriaAttr::ColCount),
        );
    }

    #[test]
    fn apply_to_sets_string_value_on_attr_map() {
        use ars_core::{AriaAttr, AttrMap, HtmlAttr};

        let mut attrs = AttrMap::new();

        AriaAttribute::Disabled(true).apply_to(&mut attrs);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"),);
    }

    #[test]
    fn apply_to_removes_nullable_absent_attrs() {
        use ars_core::{AriaAttr, AttrMap, HtmlAttr};

        let mut attrs = AttrMap::new();

        // Pre-set to verify removal
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        AriaAttribute::Hidden(None).apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
    }

    #[test]
    fn apply_to_pressed_none_removes_attr() {
        use ars_core::{AriaAttr, AttrMap, HtmlAttr};

        let mut attrs = AttrMap::new();

        AriaAttribute::Pressed(None).apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Pressed)));
    }

    #[test]
    fn apply_to_selected_none_removes_attr() {
        use ars_core::{AriaAttr, AttrMap, HtmlAttr};

        let mut attrs = AttrMap::new();

        AriaAttribute::Selected(None).apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Selected)));
    }

    #[test]
    fn apply_to_label_string() {
        use ars_core::{AriaAttr, AttrMap, HtmlAttr};

        let mut attrs = AttrMap::new();

        AriaAttribute::Label("Close dialog".into()).apply_to(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Close dialog"),
        );
    }

    #[test]
    fn from_aria_attr_produces_default_values() {
        use ars_core::AriaAttr;

        assert_eq!(
            AriaAttribute::from(AriaAttr::Disabled),
            AriaAttribute::Disabled(false),
        );
        assert_eq!(
            AriaAttribute::from(AriaAttr::ActiveDescendant),
            AriaAttribute::ActiveDescendant(None),
        );
        assert_eq!(
            AriaAttribute::from(AriaAttr::Checked),
            AriaAttribute::Checked(AriaChecked::False),
        );
        assert_eq!(
            AriaAttribute::from(AriaAttr::ColCount),
            AriaAttribute::ColCount(-1),
        );
        assert_eq!(
            AriaAttribute::from(AriaAttr::Level),
            AriaAttribute::Level(1),
        );
    }

    #[test]
    fn from_aria_attr_covers_all_noncompat_variants() {
        let cases = [
            (
                AriaAttr::ActiveDescendant,
                AriaAttribute::ActiveDescendant(None),
            ),
            (
                AriaAttr::AutoComplete,
                AriaAttribute::AutoComplete(AriaAutocomplete::None),
            ),
            (
                AriaAttr::Checked,
                AriaAttribute::Checked(AriaChecked::False),
            ),
            (
                AriaAttr::Controls,
                AriaAttribute::Controls(AriaIdList::default()),
            ),
            (
                AriaAttr::Current,
                AriaAttribute::Current(AriaCurrent::False),
            ),
            (
                AriaAttr::DescribedBy,
                AriaAttribute::DescribedBy(AriaIdList::default()),
            ),
            (
                AriaAttr::Description,
                AriaAttribute::Description(String::new()),
            ),
            (
                AriaAttr::Details,
                AriaAttribute::Details(AriaIdRef(String::new())),
            ),
            (AriaAttr::Disabled, AriaAttribute::Disabled(false)),
            (
                AriaAttr::FlowTo,
                AriaAttribute::FlowTo(AriaIdList::default()),
            ),
            (
                AriaAttr::HasPopup,
                AriaAttribute::HasPopup(AriaHasPopup::False),
            ),
            (AriaAttr::Hidden, AriaAttribute::Hidden(Some(true))),
            (
                AriaAttr::Invalid,
                AriaAttribute::Invalid(AriaInvalid::False),
            ),
            (AriaAttr::Label, AriaAttribute::Label(String::new())),
            (
                AriaAttr::LabelledBy,
                AriaAttribute::LabelledBy(AriaIdList::default()),
            ),
            (AriaAttr::Level, AriaAttribute::Level(1)),
            (AriaAttr::Live, AriaAttribute::Live(AriaLive::Off)),
            (AriaAttr::Modal, AriaAttribute::Modal(false)),
            (AriaAttr::MultiLine, AriaAttribute::MultiLine(false)),
            (
                AriaAttr::MultiSelectable,
                AriaAttribute::MultiSelectable(false),
            ),
            (
                AriaAttr::Orientation,
                AriaAttribute::Orientation(AriaOrientation::Horizontal),
            ),
            (AriaAttr::Owns, AriaAttribute::Owns(AriaIdList::default())),
            (
                AriaAttr::Placeholder,
                AriaAttribute::Placeholder(String::new()),
            ),
            (AriaAttr::PosInSet, AriaAttribute::PosInSet(1)),
            (
                AriaAttr::Pressed,
                AriaAttribute::Pressed(Some(AriaPressed::False)),
            ),
            (AriaAttr::ReadOnly, AriaAttribute::ReadOnly(false)),
            (AriaAttr::Required, AriaAttribute::Required(false)),
            (
                AriaAttr::RoleDescription,
                AriaAttribute::RoleDescription(String::new()),
            ),
            (AriaAttr::Selected, AriaAttribute::Selected(Some(false))),
            (AriaAttr::SetSize, AriaAttribute::SetSize(0)),
            (AriaAttr::Sort, AriaAttribute::Sort(AriaSort::None)),
            (AriaAttr::ValueMax, AriaAttribute::ValueMax(0.0)),
            (AriaAttr::ValueMin, AriaAttribute::ValueMin(0.0)),
            (AriaAttr::ValueNow, AriaAttribute::ValueNow(0.0)),
            (AriaAttr::ValueText, AriaAttribute::ValueText(String::new())),
            (AriaAttr::Atomic, AriaAttribute::Atomic(false)),
            (AriaAttr::Busy, AriaAttribute::Busy(false)),
            (
                AriaAttr::Relevant,
                AriaAttribute::Relevant(AriaRelevant::default()),
            ),
            (
                AriaAttr::ErrorMessage,
                AriaAttribute::ErrorMessage(AriaIdRef(String::new())),
            ),
            (AriaAttr::Expanded, AriaAttribute::Expanded(Some(false))),
            (
                AriaAttr::KeyShortcuts,
                AriaAttribute::KeyShortcuts(String::new()),
            ),
            (AriaAttr::ColCount, AriaAttribute::ColCount(-1)),
            (AriaAttr::ColIndex, AriaAttribute::ColIndex(1)),
            (AriaAttr::ColSpan, AriaAttribute::ColSpan(1)),
            (AriaAttr::RowCount, AriaAttribute::RowCount(-1)),
            (AriaAttr::RowIndex, AriaAttribute::RowIndex(1)),
            (AriaAttr::RowSpan, AriaAttribute::RowSpan(1)),
        ];

        for (attr, expected) in cases {
            assert_eq!(AriaAttribute::from(attr), expected, "{attr:?}");
        }
    }

    #[cfg(feature = "aria-drag-drop-compat")]
    #[test]
    fn from_aria_attr_covers_compat_variants() {
        assert_eq!(
            AriaAttribute::from(AriaAttr::DropEffect),
            AriaAttribute::DropEffect(AriaDropeffect::None)
        );
        assert_eq!(
            AriaAttribute::from(AriaAttr::Grabbed),
            AriaAttribute::Grabbed(None)
        );
    }

    #[test]
    fn from_aria_attribute_ref_extracts_discriminant() {
        use ars_core::AriaAttr;

        assert_eq!(
            AriaAttr::from(&AriaAttribute::Disabled(true)),
            AriaAttr::Disabled,
        );
        assert_eq!(
            AriaAttr::from(&AriaAttribute::Label("hello".into())),
            AriaAttr::Label,
        );
        assert_eq!(
            AriaAttr::from(&AriaAttribute::RowSpan(3)),
            AriaAttr::RowSpan,
        );
    }

    #[test]
    fn try_from_html_attr_aria_succeeds() {
        use ars_core::{AriaAttr, HtmlAttr};

        let result = AriaAttribute::try_from(HtmlAttr::Aria(AriaAttr::Busy));

        assert!(result.is_ok());
        assert_eq!(result.expect("should be Ok"), AriaAttribute::Busy(false));
    }

    #[test]
    fn try_from_html_attr_non_aria_fails() {
        use ars_core::HtmlAttr;

        let result = AriaAttribute::try_from(HtmlAttr::Class);

        assert_eq!(result, Err(HtmlAttr::Class));
    }

    #[test]
    fn round_trip_discriminant_preserves_identity() {
        use ars_core::AriaAttr;

        // AriaAttr → AriaAttribute → AriaAttr round-trip
        let original = AriaAttr::Orientation;

        let typed = AriaAttribute::from(original);

        let back = AriaAttr::from(&typed);

        assert_eq!(original, back);
    }

    #[test]
    fn representative_attributes_round_trip_to_discriminants() {
        let attrs = [
            AriaAttribute::ActiveDescendant(Some(AriaIdRef("active".into()))),
            AriaAttribute::AutoComplete(AriaAutocomplete::Inline),
            AriaAttribute::Checked(AriaChecked::Mixed),
            AriaAttribute::Controls(AriaIdList(vec!["a".into(), "b".into()])),
            AriaAttribute::Current(AriaCurrent::Page),
            AriaAttribute::DescribedBy(AriaIdList(vec!["help".into()])),
            AriaAttribute::Description("desc".into()),
            AriaAttribute::Details(AriaIdRef("details".into())),
            AriaAttribute::Disabled(true),
            AriaAttribute::FlowTo(AriaIdList(vec!["next".into()])),
            AriaAttribute::HasPopup(AriaHasPopup::Dialog),
            AriaAttribute::Hidden(Some(true)),
            AriaAttribute::Invalid(AriaInvalid::Grammar),
            AriaAttribute::Label("label".into()),
            AriaAttribute::LabelledBy(AriaIdList(vec!["label".into()])),
            AriaAttribute::Level(2),
            AriaAttribute::Live(AriaLive::Polite),
            AriaAttribute::Modal(true),
            AriaAttribute::MultiLine(true),
            AriaAttribute::MultiSelectable(true),
            AriaAttribute::Orientation(AriaOrientation::Vertical),
            AriaAttribute::Owns(AriaIdList(vec!["child".into()])),
            AriaAttribute::Placeholder("type here".into()),
            AriaAttribute::PosInSet(7),
            AriaAttribute::Pressed(Some(AriaPressed::Mixed)),
            AriaAttribute::ReadOnly(true),
            AriaAttribute::Required(true),
            AriaAttribute::RoleDescription("switch".into()),
            AriaAttribute::Selected(Some(true)),
            AriaAttribute::SetSize(12),
            AriaAttribute::Sort(AriaSort::Other),
            AriaAttribute::ValueMax(10.0),
            AriaAttribute::ValueMin(1.0),
            AriaAttribute::ValueNow(4.0),
            AriaAttribute::ValueText("four".into()),
            AriaAttribute::Atomic(true),
            AriaAttribute::Busy(true),
            AriaAttribute::Relevant(AriaRelevant::default()),
            AriaAttribute::ErrorMessage(AriaIdRef("error".into())),
            AriaAttribute::Expanded(Some(true)),
            AriaAttribute::KeyShortcuts("Ctrl+P".into()),
            AriaAttribute::ColCount(3),
            AriaAttribute::ColIndex(2),
            AriaAttribute::ColSpan(2),
            AriaAttribute::RowCount(5),
            AriaAttribute::RowIndex(4),
            AriaAttribute::RowSpan(3),
        ];

        for attr in attrs {
            let key = AriaAttr::from(&attr);

            assert_eq!(attr.to_html_attr(), HtmlAttr::Aria(key), "{attr:?}");
            assert_eq!(
                AriaAttribute::from(key).attr_name(),
                attr.attr_name(),
                "{attr:?}"
            );
        }
    }
}
