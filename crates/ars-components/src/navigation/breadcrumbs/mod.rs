//! Breadcrumb navigation component.
//!
//! Breadcrumbs is a stateless connect API for ordered navigation trails. It
//! owns landmark/list/current-page attributes, separator text, collapsed-tail
//! layout calculation, and URL sanitization for rendered crumb links.

use alloc::string::{String, ToString as _};
use core::ops::Range;

use ars_core::{
    AriaAttr, AttrMap, ComponentPart, ConnectApi, Direction, HtmlAttr, SafeUrl, sanitize_url,
};

use super::link::AriaCurrent;

/// Visual separator token rendered between breadcrumb items.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Separator {
    /// The slash separator (`/`).
    #[default]
    Slash,

    /// A chevron-like separator (`>`).
    Chevron,

    /// Consumer-provided separator text.
    Custom(String),
}

impl Separator {
    /// Returns the text that should be rendered inside the separator part.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Slash => "/",
            Self::Chevron => ">",
            Self::Custom(value) => value.as_str(),
        }
    }
}

/// A consumer-provided breadcrumb item.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemDef {
    /// Visible item label.
    pub label: String,

    /// Optional safe navigation target. Current items typically omit it.
    pub href: Option<SafeUrl>,

    /// Optional current-item semantic override.
    pub current: Option<AriaCurrent>,
}

impl ItemDef {
    /// Creates a breadcrumb item with the supplied visible label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
            current: None,
        }
    }

    /// Sets the item's navigation target.
    #[must_use]
    pub fn href(mut self, href: SafeUrl) -> Self {
        self.href = Some(href);
        self
    }

    /// Sets the item's current semantic.
    #[must_use]
    pub const fn current(mut self, current: AriaCurrent) -> Self {
        self.current = Some(current);
        self
    }
}

/// Immutable configuration for a [`Breadcrumbs`](self) instance.
#[derive(Clone, Debug, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Separator text strategy rendered between visible items.
    pub separator: Separator,

    /// Text direction applied to the navigation landmark.
    pub dir: Direction,

    /// Localized accessible label for the navigation landmark.
    pub nav_label: String,

    /// Optional maximum number of visible items before the middle collapses.
    pub max_items: Option<usize>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            separator: Separator::default(),
            dir: Direction::Ltr,
            nav_label: "Breadcrumb".to_string(),
            max_items: None,
        }
    }
}

impl Props {
    /// Returns default breadcrumb props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`separator`](Self::separator).
    #[must_use]
    pub fn separator(mut self, separator: Separator) -> Self {
        self.separator = separator;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = dir;
        self
    }

    /// Sets [`nav_label`](Self::nav_label).
    #[must_use]
    pub fn nav_label(mut self, label: impl Into<String>) -> Self {
        self.nav_label = label.into();
        self
    }

    /// Sets [`max_items`](Self::max_items).
    #[must_use]
    pub const fn max_items(mut self, max_items: Option<usize>) -> Self {
        self.max_items = max_items;
        self
    }
}

/// Anatomy parts exposed by the breadcrumb connect API.
#[derive(ComponentPart)]
#[scope = "breadcrumbs"]
pub enum Part {
    /// Navigation landmark root.
    Root,

    /// Ordered list wrapper.
    List,

    /// A single breadcrumb list item.
    Item,

    /// A non-current navigation link.
    Link {
        /// Link target used by [`ConnectApi::part_attrs`].
        #[part(default = SafeUrl::from_static("/"))]
        href: SafeUrl,
    },

    /// Current-page item.
    CurrentPage,

    /// Decorative separator between visible items.
    Separator,
}

/// Layout decision returned by [`Api::layout`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreadcrumbLayout {
    /// Every breadcrumb item should be rendered.
    Full,

    /// The middle range should be replaced by an ellipsis trigger.
    Collapsed {
        /// The first visible item index.
        first_index: usize,

        /// The hidden range represented by the ellipsis trigger.
        ellipsis_replaces: Range<usize>,

        /// The visible tail range.
        visible_tail: Range<usize>,
    },
}

/// Stateless API for rendering breadcrumb attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Api {
    /// Immutable props backing this API.
    props: Props,
}

impl Api {
    /// Creates a stateless breadcrumb API from props.
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Borrows the props backing this API.
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Attributes for the navigation landmark.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                self.props.nav_label.as_str(),
            )
            .set(HtmlAttr::Dir, self.props.dir.as_html_attr());

        attrs
    }

    /// Attributes for the ordered list.
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::List.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Role, "list");

        attrs
    }

    /// Attributes for a list item.
    #[must_use]
    pub fn item_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Item.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value);

        attrs
    }

    /// Attributes for a breadcrumb link.
    #[must_use]
    pub fn link_attrs(&self, href: &SafeUrl) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Link {
            // Inert placeholder: `data_attrs()` only yields the scope/part
            // tokens (the real href is set below). Matches the `#[part]`
            // default and the spec anatomy so the literal stays consistent.
            href: SafeUrl::from_static("/"),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Href, sanitize_url(href.as_str()));

        attrs
    }

    /// Attributes for the current page item.
    #[must_use]
    pub fn current_page_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::CurrentPage.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Aria(AriaAttr::Current), "page");

        attrs
    }

    /// Attributes for a decorative separator.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Separator.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns the configured separator text.
    #[must_use]
    pub const fn separator_text(&self) -> &str {
        self.props.separator.as_str()
    }

    /// Computes the visible item layout for the supplied item count.
    #[must_use]
    pub const fn layout(&self, total_items: usize) -> BreadcrumbLayout {
        if let Some(max_items) = self.props.max_items
            && (total_items > max_items && max_items >= 2)
        {
            let visible_end_count = max_items - 1;

            let tail_start = total_items.saturating_sub(visible_end_count);

            BreadcrumbLayout::Collapsed {
                first_index: 0,
                ellipsis_replaces: 1..tail_start,
                visible_tail: tail_start..total_items,
            }
        } else {
            BreadcrumbLayout::Full
        }
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item => self.item_attrs(),
            Part::Link { href } => self.link_attrs(&href),
            Part::CurrentPage => self.current_page_attrs(),
            Part::Separator => self.separator_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn root_list_item_current_and_separator_attrs_match_spec() {
        let api = Api::new(
            Props::new()
                .id("crumbs")
                .dir(Direction::Rtl)
                .nav_label("Trail"),
        );

        insta::assert_snapshot!("breadcrumbs_root", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!("breadcrumbs_list", snapshot_attrs(&api.list_attrs()));
        insta::assert_snapshot!("breadcrumbs_item", snapshot_attrs(&api.item_attrs()));
        insta::assert_snapshot!(
            "breadcrumbs_current_page",
            snapshot_attrs(&api.current_page_attrs())
        );
        insta::assert_snapshot!(
            "breadcrumbs_separator",
            snapshot_attrs(&api.separator_attrs())
        );
    }

    #[test]
    fn link_attrs_emit_sanitized_safe_href() {
        let api = Api::new(Props::new());

        let attrs = api.link_attrs(&SafeUrl::from_static("/products"));

        assert_eq!(attrs.get(&HtmlAttr::Href), Some("/products"));
        insta::assert_snapshot!("breadcrumbs_link", snapshot_attrs(&attrs));
    }

    #[test]
    fn separator_text_uses_configured_token() {
        assert_eq!(
            Api::new(Props::new().separator(Separator::Chevron)).separator_text(),
            ">"
        );
        assert_eq!(
            Api::new(Props::new().separator(Separator::Custom("::".into()))).separator_text(),
            "::"
        );
    }

    #[test]
    fn collapsed_layout_keeps_first_and_tail() {
        let api = Api::new(Props::new().max_items(Some(4)));

        assert_eq!(
            api.layout(7),
            BreadcrumbLayout::Collapsed {
                first_index: 0,
                ellipsis_replaces: 1..4,
                visible_tail: 4..7,
            }
        );
        assert_eq!(api.layout(4), BreadcrumbLayout::Full);
        assert_eq!(
            Api::new(Props::new().max_items(Some(1))).layout(7),
            BreadcrumbLayout::Full
        );
    }

    #[test]
    fn item_def_records_label_href_and_current_semantics() {
        let item = ItemDef::new("Checkout")
            .href(SafeUrl::from_static("/checkout"))
            .current(AriaCurrent::Step);

        assert_eq!(item.label, "Checkout");
        assert_eq!(item.href.as_ref().map(SafeUrl::as_str), Some("/checkout"));
        assert_eq!(item.current, Some(AriaCurrent::Step));
    }

    #[test]
    fn props_builder_sets_every_field() {
        let props = Props::new()
            .id("crumbs")
            .separator(Separator::Chevron)
            .dir(Direction::Rtl)
            .nav_label("Trail")
            .max_items(Some(3));

        assert_eq!(props.id, "crumbs");
        assert_eq!(props.separator, Separator::Chevron);
        assert_eq!(props.dir, Direction::Rtl);
        assert_eq!(props.nav_label, "Trail");
        assert_eq!(props.max_items, Some(3));
    }

    #[test]
    fn part_attrs_dispatches_every_part() {
        let api = Api::new(Props::new());

        let href = SafeUrl::from_static("/products");

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::List), api.list_attrs());
        assert_eq!(api.part_attrs(Part::Item), api.item_attrs());
        assert_eq!(
            api.part_attrs(Part::Link { href: href.clone() }),
            api.link_attrs(&href)
        );
        assert_eq!(api.part_attrs(Part::CurrentPage), api.current_page_attrs());
        assert_eq!(api.part_attrs(Part::Separator), api.separator_attrs());
    }
}
