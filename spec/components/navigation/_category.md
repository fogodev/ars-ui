# Navigation Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and crate structure,
`03-accessibility.md` for ARIA patterns, focus management, and keyboard navigation,
`04-internationalization.md` for RTL support and locale-aware text,
`05-interactions.md` for keyboard and pointer handling.

---

## Table of Contents

- [Accordion](accordion.md)
- [Tabs](tabs.md)
- [TreeView](tree-view.md)
- [Pagination](pagination.md)
- [Steps](steps.md)
- [Breadcrumbs](breadcrumbs.md)
- [`Link`](link.md)

---

## Overview

Navigation components allow users to traverse content hierarchies, multi-page sets, and
wizard flows. They span collapsible sections (`Accordion`), tabbed interfaces (`Tabs`),
hierarchical trees (`TreeView`), page navigation (`Pagination`), multi-step wizards (`Steps`),
trail navigation (`Breadcrumbs`), and accessible links (`Link`).

| Component     | Purpose                                                                           |
| ------------- | --------------------------------------------------------------------------------- |
| `Accordion`   | Expandable/collapsible panel group with single or multiple open items             |
| `Tabs`        | Tab list with associated content panels; supports automatic and manual activation |
| `TreeView`    | Hierarchical tree with expand/collapse and optional node selection                |
| `Pagination`  | Page navigation controls with ellipsis-based range generation                     |
| `Steps`       | Multi-step wizard progress indicator with per-step status                         |
| `Breadcrumbs` | Path navigation trail; no state machine, pure DOM props                           |
| `Link`        | Accessible link with client-side router integration and external link detection   |

All stateful navigation components follow the standard ars-ui machine pattern:

- Zero framework dependencies — all logic lives in `ars-core`.
- `Bindable<T>` handles controlled and uncontrolled values identically.
- Each component defines a `Part` enum with `#[derive(ComponentPart)]` and implements `ConnectApi` with `fn part_attrs()` dispatching to per-part `*_attrs()` methods returning `AttrMap`.
- Data attributes on every part (`data-ars-scope`, `data-ars-part`, `data-ars-state`, etc.) enable
  CSS-first styling without class gymnastics.
- `#[derive(Clone, Debug, PartialEq)]` is applied wherever comparison or debug output is useful.

---

## Landmark Semantics

Navigation components should use appropriate landmark semantics for assistive technology:

1. **MenuBar**: Wrapping element uses `role="menubar"`. Parent container should be wrapped in `<nav aria-label="Main navigation">` when used as primary navigation.
2. **Breadcrumbs**: Use `<nav aria-label="Breadcrumb">` wrapper with `<ol>` list. Current page item has `aria-current="page"`.
3. **Navigation**: All navigation components recommend `<nav>` wrapper with descriptive `aria-label` to distinguish multiple nav landmarks.
4. **Skip-to-content**: Adapters should provide guidance for adding a skip link as the first focusable element on the page, targeting the main content landmark.

## `Navbar` and `MenuBar` Role Documentation

### `Navbar`

Navbar renders with `role="navigation"` and requires an `aria-label` (e.g., `"Main navigation"`). This identifies the landmark for screen reader navigation. When multiple `<nav>` elements exist on a page, each MUST have a unique `aria-label` to distinguish them.

```rust
/// Get root attributes for `Navbar`.
pub fn root_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "navigation");
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), &self.props.label); // e.g., "Main navigation"
    attrs
}
```

### MenuBar

MenuBar renders with `role="menubar"` and uses `role="menuitem"` for its children. Menu items that open submenus use `aria-haspopup="true"` and `aria-expanded`.

```rust
/// Get root attributes for `MenuBar`.
pub fn root_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "menubar");
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), &self.props.label);
    attrs
}

/// Get item attributes for `MenuBar`.
pub fn item_attrs(&self, item_id: &str) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item(Default::default()).data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "menuitem");
    attrs.set(HtmlAttr::TabIndex, if self.is_first(item_id) { "0" } else { "-1" });
    attrs
}
```

---

## Summary

| Component     | Machine States               | Key Context Fields                                                                                                                                | Stateful Value                         |
| ------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `Accordion`   | `Idle`                       | `value: Bindable<BTreeSet<Key>>`, `multiple`, `collapsible`, `orientation`                                                                        | Open item IDs                          |
| `Tabs`        | `Idle`, `Focused { tab }`    | `value: Bindable<String>`, `activation_mode`, `dir`, `loop_focus`                                                                                 | Selected tab ID                        |
| `TreeView`    | `Idle`, `Focused`            | `items: TreeCollection<TreeItem>`, `selected: Bindable<selection::Set>`, `expanded: Bindable<BTreeSet<Key>>`, `selection_state: selection::State` | Tree data + selected + expanded        |
| `Pagination`  | `Idle`                       | `page: Bindable<u32>`, `page_size`, `total_items`, `sibling_count`, `page_count`                                                                  | Current page number                    |
| `Steps`       | `Idle`                       | `step: Bindable<u32>`, `count`, `statuses: Vec<steps::Status>`, `linear`, `orientation`                                                           | Current step index + per-step statuses |
| `Breadcrumbs` | (no machine)                 | (stateless — Props only: `separator`, `dir`, `nav_label`)                                                                                         | (stateless)                            |
| `Link`        | `Idle`, `Focused`, `Pressed` | `href`, `target`, `rel`, `is_current: Option<AriaCurrent>`, `disabled`, `focus_visible`                                                           | (stateless — no bindable value)        |

All seven components expose their parts through consistent data attributes:

- `data-ars-scope="{component}"` — identifies the component namespace on every part element.
- `data-ars-part="{part}"` — identifies the specific anatomy part.
- `data-ars-state="{value}"` — communicates the current state to CSS and test selectors.
- `data-ars-disabled`, `data-ars-selected`, `data-ars-expanded`, `data-ars-current`,
  `data-ars-focus-visible`, `data-ars-orientation`, `data-ars-index` — supplementary
  state attributes used where applicable.
