---
component: Breadcrumbs
category: navigation
tier: stateless
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
  react-aria: Breadcrumbs
---

# Breadcrumbs

A path navigation trail showing the user's location in a site hierarchy. `Breadcrumb`s are
a simple, stateless component — they require no state machine, only an `Api` returning
DOM props.

**Anatomy overview**: `Breadcrumb`s consists of six anatomy parts: **Root** (`<nav>`, `role="navigation"`, `aria-label`), **List** (`<ol>`, `role="list"`), **Item** (`<li>`), **Link** (`<a>`, standard link semantics), **CurrentPage** (`<span>`, `aria-current="page"`), and **Separator** (`<span>`, `aria-hidden="true"`). The last item is a non-interactive CurrentPage instead of a link. Separators are purely decorative and hidden from assistive technologies.

## 1. API

### 1.1 Props

```rust
use ars_i18n::Direction;

/// The type of separator to use between breadcrumb items.
#[derive(Clone, Debug, PartialEq)]
pub enum Separator {
    /// "/" character (default).
    Slash,
    /// "›" or similar chevron character.
    Chevron,
    /// Custom string, e.g. "•" or a locale-specific separator.
    Custom(String),
}

impl Separator {
    /// Returns the separator string to render inside the separator element.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Slash         => "/",
            Self::Chevron       => "›",
            Self::Custom(s)     => s.as_str(),
        }
    }
}

/// Props for the `Breadcrumbs` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// The type of separator to use between breadcrumb items.
    pub separator: Separator,
    /// The text direction; affects visual order in RTL layouts.
    pub dir: Direction,
    /// Localized label for the `<nav>` landmark.
    pub nav_label: String,
    /// Maximum number of items to display before collapsing. When `Some(n)`,
    /// if the item count exceeds `n`, the middle items are replaced with an
    /// ellipsis menu that expands on click. `None` displays all items.
    /// Default: `None`.
    pub max_items: Option<usize>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            separator: Separator::Slash,
            dir: Direction::Ltr,
            nav_label: "Breadcrumb".to_string(),
            max_items: None,
        }
    }
}
```

### 1.2 Connect / API

```rust
use ars_core::AttrMap;

#[derive(ComponentPart)]
#[scope = "breadcrumbs"]
pub enum Part {
    Root,
    List,
    Item,
    Link { href: String },
    CurrentPage,
    Separator,
}

/// API for the `Breadcrumbs` component.
pub struct Api {
    /// The props of the component.
    props: Props,
}

impl Api {
    /// Breadcrumbs has no state machine — it is a stateless component.
    /// `new()` constructs the Api directly from Props.
    pub fn new(props: Props) -> Self {
        Api { props }
    }

    /// Attrs for the `<nav>` landmark element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), &self.props.nav_label);
        attrs.set(HtmlAttr::Dir, match self.props.dir {
            Direction::Ltr  => "ltr",
            Direction::Rtl  => "rtl",
            Direction::Auto => "auto",
        });
        attrs
    }

    /// Attrs for the ordered list element (`<ol>`).
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "list");
        attrs
    }

    /// Attrs for a list item (`<li>`).
    pub fn item_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attrs for a navigation link (`<a>`).
    ///
    /// `href` — the target URL for this crumb.
    pub fn link_attrs(&self, href: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Link { href: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Href, href);
        attrs
    }

    /// Attrs for the final, non-linked item representing the current page.
    pub fn current_page_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CurrentPage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Current), "page");
        attrs
    }

    /// Attrs for the visual separator element between items.
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Returns the separator string to render inside the separator element.
    pub fn separator_text(&self) -> &str {
        self.props.separator.as_str()
    }

    /// Compute which item indices to display when `max_items` is set.
    ///
    /// When `total_items <= max_items` (or `max_items` is `None`), returns
    /// `BreadcrumbLayout::Full` — all items are visible.
    ///
    /// When `total_items > max_items`, returns `BreadcrumbLayout::Collapsed`
    /// with the first item, the last `(max_items - 1)` items, and an ellipsis
    /// position between them. The adapter renders a button at the ellipsis
    /// position that expands the collapsed items on click.
    pub fn layout(&self, total_items: usize) -> BreadcrumbLayout {
        match self.props.max_items {
            Some(max) if total_items > max && max >= 2 => {
                let visible_end_count = max - 1; // Reserve 1 slot for the first item.
                let start = total_items - visible_end_count;
                BreadcrumbLayout::Collapsed {
                    first_index: 0,
                    ellipsis_replaces: 1..start,
                    visible_tail: start..total_items,
                }
            }
            _ => BreadcrumbLayout::Full,
        }
    }
}

/// Layout result from `Api::layout()`.
#[derive(Clone, Debug, PartialEq)]
pub enum BreadcrumbLayout {
    /// All items are visible — render every index in order.
    Full,
    /// Items are collapsed. Render `first_index`, then an ellipsis trigger
    /// (which expands `ellipsis_replaces` on click), then `visible_tail`.
    Collapsed {
        first_index: usize,
        ellipsis_replaces: core::ops::Range<usize>,
        visible_tail: core::ops::Range<usize>,
    },
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item => self.item_attrs(),
            Part::Link { ref href } => self.link_attrs(href),
            Part::CurrentPage => self.current_page_attrs(),
            Part::Separator => self.separator_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Breadcrumbs
└── Root               <nav> aria-label="Breadcrumb"
    └── List           <ol> role="list"
        ├── Item (×N)  <li>
        │   ├── Link   <a href="...">
        │   └── Separator   aria-hidden="true"
        └── Item (last)
            └── CurrentPage  <span> aria-current="page"
```

| Part          | Element  | Key Attributes                                                                          |
| ------------- | -------- | --------------------------------------------------------------------------------------- |
| `Root`        | `<nav>`  | `data-ars-scope="breadcrumbs"`, `data-ars-part="root"`, `aria-label` (localized), `dir` |
| `List`        | `<ol>`   | `data-ars-scope="breadcrumbs"`, `data-ars-part="list"`, `role="list"`                   |
| `Item`        | `<li>`   | `data-ars-scope="breadcrumbs"`, `data-ars-part="item"`                                  |
| `Link`        | `<a>`    | `data-ars-scope="breadcrumbs"`, `data-ars-part="link"`, `href`                          |
| `CurrentPage` | `<span>` | `data-ars-scope="breadcrumbs"`, `data-ars-part="current-page"`, `aria-current="page"`   |
| `Separator`   | `<span>` | `data-ars-scope="breadcrumbs"`, `data-ars-part="separator"`, `aria-hidden="true"`       |

### 2.1 Current Page Semantics

The last item in the `Breadcrumbs` list represents the current page and receives special treatment:

- **`aria-current="page"`** is set on the current (last) breadcrumb item's element.
- The current item is rendered as plain text (not a link), since navigating to the current page is redundant.
- If the consumer explicitly marks an item as current via `is_current: true` on a non-last item (e.g., for multi-step flows), that item receives `aria-current="step"` instead.
- Screen readers announce the current item distinctly (e.g., "Home, link / Products, link / Shoes, current page").

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part          | Role                       | Properties                                              |
| ------------- | -------------------------- | ------------------------------------------------------- |
| `Root`        | `navigation` (via `<nav>`) | `aria-label="Breadcrumb"` (localized)                   |
| `List`        | `list`                     | —                                                       |
| `Link`        | `link` (native `<a>`)      | Standard link semantics; no additional ARIA needed      |
| `CurrentPage` | (none / `<span>`)          | `aria-current="page"`                                   |
| `Separator`   | (none)                     | `aria-hidden="true"` — separators are purely decorative |

Screen readers announce the breadcrumb trail as a navigation landmark. The `<ol>` element
communicates the ordered, hierarchical nature of the trail. The current page item does not
use `<a>` (it is not a link) and receives `aria-current="page"` so screen readers announce
it as the current location.

### 3.2 Keyboard Interaction

`Breadcrumbs` use standard link and navigation semantics — no custom keyboard handling is required.
Users `Tab` through the links in order; the current page item is not focusable (not an `<a>` tag).

## 4. Internationalization

- **`aria-label`**: The `"Breadcrumb"` label on the `<nav>` element is a translation key
  (`breadcrumbs.nav_label`) supplied by `ars-i18n`. The `Props::nav_label` field
  holds the resolved string.
- **Separator character**: Locale-specific separators (e.g., `‹` in some RTL conventions,
  or `>` in Western UIs) are selected via `Separator::Custom`. RTL locales may
  prefer `‹` over `›`.
- **RTL visual order**: In RTL layouts the breadcrumb trail reads right-to-left visually.
  Setting `dir="rtl"` on the Root element causes the browser to reverse the flex/inline
  direction automatically; no JavaScript reordering is needed. The logical order of items
  in the DOM remains first-crumb-first for screen readers.
- **Text content**: All link labels and the current page label are consumer-provided.

## 5. Library Parity

> Compared against: React Aria (`Breadcrumbs`).

### 5.1 Props

| Feature                | ars-ui                             | React Aria           | Notes                                       |
| ---------------------- | ---------------------------------- | -------------------- | ------------------------------------------- |
| Items (dynamic)        | Consumer-provided via adapter      | `items: Iterable<T>` | ars-ui is stateless; adapter iterates items |
| Disabled               | --                                 | `isDisabled`         | See below                                   |
| Separator              | `separator` (Slash/Chevron/Custom) | --                   | ars-ui addition                             |
| Dir                    | `dir`                              | --                   | ars-ui addition                             |
| Nav label              | `nav_label`                        | --                   | ars-ui addition                             |
| Max items (collapsing) | `max_items`                        | --                   | ars-ui addition                             |

**Gaps:**

- **`isDisabled`**: React Aria supports disabling all breadcrumbs globally. This is a low-value feature for breadcrumbs (disabling navigation links is unusual). Consumers can achieve this by omitting `href` attributes. Not recommended for adoption.

### 5.2 Anatomy

| Part         | ars-ui           | React Aria              | Notes                                |
| ------------ | ---------------- | ----------------------- | ------------------------------------ |
| Root         | `Root` (`<nav>`) | `Breadcrumbs`           | Full match                           |
| List         | `List` (`<ol>`)  | -- (implicit)           | ars-ui explicit `<ol>` for semantics |
| Item         | `Item` (`<li>`)  | `Breadcrumb`            | Full match                           |
| Link         | `Link` (`<a>`)   | `Link` (nested)         | Full match                           |
| Current page | `CurrentPage`    | `isCurrent` render prop | Full match                           |
| Separator    | `Separator`      | --                      | ars-ui addition                      |

**Gaps:** None.

### 5.3 Events

| Callback | ars-ui | React Aria      | Notes     |
| -------- | ------ | --------------- | --------- |
| Action   | --     | `onAction(Key)` | See below |

**Gaps:**

- **`onAction`**: React Aria fires `onAction(Key)` when a breadcrumb is clicked. ars-ui uses standard `<a>` link navigation, so no callback is needed -- the browser handles navigation. For client-side routing, the adapter intercepts clicks. Not a gap.

### 5.4 Features

| Feature                          | ars-ui             | React Aria |
| -------------------------------- | ------------------ | ---------- |
| aria-current="page" on last item | Yes                | Yes        |
| Navigation landmark              | Yes (`<nav>`)      | Yes        |
| Ordered list semantics           | Yes (`<ol>`)       | Yes        |
| Separator decoration             | Yes (configurable) | No         |
| Collapsed breadcrumbs (ellipsis) | Yes (`max_items`)  | No         |
| RTL support                      | Yes                | No         |
| Disabled state                   | No                 | Yes        |

**Gaps:** None worth adopting (see `isDisabled` note above).

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui is a stateless component with no state machine, while React Aria uses collection-based items. ars-ui adds separator customization, collapsed breadcrumbs, and RTL support beyond React Aria.
- **Recommended additions:** None.
