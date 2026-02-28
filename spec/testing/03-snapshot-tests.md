# Snapshot Tests

## 1. AttrMap Snapshot Tests for `connect()`

The `connect()` / `Api` layer produces `AttrMap` values that must match exact ARIA contracts. Snapshot tests catch regressions in attribute output.

### 1.1 Pattern with `insta`

```rust
#[cfg(test)]
mod connect_tests {
    use super::*;
    use insta::assert_snapshot;

    /// Helper to render a button's root AttrMap as a Debug string for snapshot comparison.
    /// Use a concrete machine type (here `button::Machine`) to avoid ambiguous type inference.
    fn make_api(state: button::State, ctx: button::Context) -> String {
        let props = button::Props::default();
        let send = |_: button::Event| {};
        let api = button::Machine::connect(&state, &ctx, &props, &send);
        // Serialize the root AttrMap to a deterministic string.
        format!("{:#?}", api.root_attrs())
    }

    #[test]
    fn button_idle_attrs() {
        let ctx = button::Context {
            disabled: false,
            loading: false,
            pressed: false,
            focused: false,
            focus_visible: false,
            variant: Some("primary".into()),
            size: Some("md".into()),
        };
        assert_snapshot!("button_idle", make_api(button::State::Idle, ctx));
    }

    #[test]
    fn button_loading_attrs() {
        let ctx = button::Context {
            disabled: false,
            loading: true,
            pressed: false,
            focused: false,
            focus_visible: false,
            variant: None,
            size: None,
        };
        assert_snapshot!("button_loading", make_api(button::State::Loading, ctx));
    }

    #[test]
    fn dialog_open_attrs() {
        let ctx = dialog::Context { open: true, ..Default::default() };
        let api = dialog::Machine::connect(&dialog::State::Open, &ctx, &dialog::Props::default(), &|_: dialog::Event| {});
        assert_snapshot!("dialog_open_backdrop", format!("{:#?}", api.backdrop_attrs()));
        assert_snapshot!("dialog_open_content", format!("{:#?}", api.content_attrs()));
    }
}
```

### 1.2 What to snapshot

- `data-ars-state` value for each machine state.
- All ARIA attributes: `role`, `aria-expanded`, `aria-selected`, `aria-disabled`, `aria-busy`, `aria-label`, etc.
- Event handler presence (handler keys, not closures).
- `tabindex` values.
- `data-ars-*` custom attributes (variant, size, scope, part).

### 1.3 Multi-Part Anatomy Snapshot Rule

**Every component MUST have snapshot tests for EACH anatomy part that produces
ARIA attributes.** A single `root_attrs()` snapshot is insufficient for
components with multi-part anatomy (e.g., Dialog has `backdrop_attrs()`,
`content_attrs()`, `title_attrs()`, `description_attrs()`, `close_trigger_attrs()`).

```rust
#[test]
fn dialog_all_anatomy_parts_snapshotted() {
    let ctx = dialog::Context { open: true, ..Default::default() };
    let api = dialog::Machine::connect(&dialog::State::Open, &ctx, &dialog::Props::default(), &|_: dialog::Event| {});

    // Every part with ARIA attributes MUST have its own snapshot
    assert_snapshot!("dialog_open_content", format!("{:#?}", api.content_attrs()));
    assert_snapshot!("dialog_open_trigger", format!("{:#?}", api.trigger_attrs()));
    assert_snapshot!("dialog_open_backdrop", format!("{:#?}", api.backdrop_attrs()));
    assert_snapshot!("dialog_open_title", format!("{:#?}", api.title_attrs()));
    assert_snapshot!("dialog_open_description", format!("{:#?}", api.description_attrs()));
    assert_snapshot!("dialog_open_close_trigger", format!("{:#?}", api.close_trigger_attrs()));
}

#[test]
fn accordion_item_anatomy_snapshotted() {
    let props = accordion::Props::default();
    let ctx = accordion::Context {
        value: Bindable::uncontrolled(BTreeSet::from([Key::from("p1")])),
        ..Default::default()
    };
    let state = accordion::State::Idle;
    let api = accordion::Machine::connect(&state, &ctx, &props, &|_: accordion::Event| {});

    // Use canonical part_attrs(Part) pattern for data-carrying variants
    let trigger_attrs = api.part_attrs(accordion::Part::ItemTrigger("p1".into(), "p1-content".into()));
    assert_snapshot!("accordion_trigger_p1", format!("{:#?}", trigger_attrs));
    let content_attrs = api.part_attrs(accordion::Part::ItemContent("p1".into(), "p1-content".into(), "p1-trigger".into()));
    assert_snapshot!("accordion_content_p1", format!("{:#?}", content_attrs));
    let item_attrs = api.part_attrs(accordion::Part::Item("p1".into()));
    assert_snapshot!("accordion_item_p1", format!("{:#?}", item_attrs));
}

#[test]
fn select_item_anatomy_snapshotted() {
    let props = select::Props::default();
    let ctx = select::Context::default();
    let send = Rc::new(|_: select::Event| {});
    let api = select::Machine::connect(&select::State::Open, &ctx, &props, &send);

    assert_snapshot!("select_trigger", format!("{:#?}", api.trigger_attrs()));
    assert_snapshot!("select_content", format!("{:#?}", api.content_attrs()));
    assert_snapshot!("select_item_selected", format!("{:#?}", api.item_attrs(&Key::from("item-1"))));
}
```

### 1.4 Snapshot Review Process

Snapshot tests are powerful regression tools but require a disciplined review process to
distinguish intentional changes from accidental regressions:

1. **Local development**: Run `cargo insta test --review` to interactively approve or reject
   snapshot changes. Never blind-accept with `cargo insta test --accept`.

2. **CI enforcement**: CI MUST reject any unapproved snapshot changes. The CI pipeline runs
   `cargo insta test` (without `--accept`) and fails if any snapshots are pending review.
   Configure with:

   ```yaml
   - run: cargo insta test --unreferenced=reject
     env:
       INSTA_UPDATE: no
   ```

3. **PR review**: Snapshot diffs (`*.snap.new` files) MUST be reviewed as part of the PR.
   Reviewers should verify that attribute changes are intentional and match the PR description.

4. **BREAKING changes**: Any snapshot change that modifies ARIA roles, removes attributes, or
   changes `data-ars-state` values MUST be documented in the changelog with a `BREAKING` label.

5. **Rollback strategy**: If a snapshot change is merged accidentally, revert the PR and
   re-run `cargo insta test --review` to restore the previous snapshots.

6. **Bulk approval for trivial updates**: When a snapshot diff contains only whitespace,
   auto-formatting, or additive (non-breaking) attribute changes, developers MAY use bulk
   approval to avoid per-snapshot fatigue. Set the `SNAPSHOT_REVIEW_MODE=bulk` environment
   variable to enable category-based batch approval:

   ```bash
   # Approve all snapshots whose diffs are whitespace-only or additive attributes
   SNAPSHOT_REVIEW_MODE=bulk cargo insta test --review
   ```

   Bulk mode groups pending snapshots by change category (whitespace, additive, breaking) and
   presents one approval prompt per category instead of per snapshot. **Breaking changes**
   (removed attributes, changed roles, altered `data-ars-state`) are NEVER auto-approved and
   always require individual review, even in bulk mode.

   > **Note:** `SNAPSHOT_REVIEW_MODE=bulk` requires a custom CI script wrapping `cargo insta review`.
   > This is not a native `insta` feature. The script must parse snapshot diffs, categorize them
   > (whitespace, additive, breaking), and present grouped approval prompts.

7. **Fast-path for trivial updates**: Snapshot changes that meet ALL of the following criteria
   may be approved without accessibility-lead sign-off:
   - The diff adds new `data-ars-*` attributes (no removals or renames).
   - No ARIA role, `aria-*`, or `tabindex` attributes are modified.
   - The change is traceable to a specific PR that documents the addition.

---

## 2. Data Attribute Stability

Tests ensuring `data-ars-*` attributes are stable, deterministic, and follow naming conventions.

### 2.1 State Value Matches Kebab-Cased Enum Variant

```rust
#[test]
fn data_ars_state_matches_kebab_case() {
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);

    // Closed state
    let api = select::Machine::connect(svc.state(), svc.context(), svc.props(), &|_| {});
    assert_eq!(api.root_attrs().get(&HtmlAttr::Data("ars-state")), Some("closed"));

    // Open state
    svc.send(select::Event::Open);
    let api = select::Machine::connect(svc.state(), svc.context(), svc.props(), &|_| {});
    assert_eq!(api.root_attrs().get(&HtmlAttr::Data("ars-state")), Some("open"));
}

#[test]
fn multi_word_state_uses_kebab_case() {
    // e.g., State::OpenPending → "open-pending"
    let props = tooltip::Props::default();
    let mut svc = Service::<tooltip::Machine>::new(props);
    svc.send(tooltip::Event::PointerEnter);

    let api = tooltip::Machine::connect(svc.state(), svc.context(), svc.props(), &|_| {});
    let state_attr = api.root_attrs().get(&HtmlAttr::Data("ars-state"));
    assert_eq!(state_attr, Some("open-pending"));
    assert!(!state_attr.expect("data-ars-state attribute confirmed present").contains('_'), "must not contain underscores");
    assert!(!state_attr.expect("data-ars-state attribute confirmed present").chars().any(|c| c.is_uppercase()), "must not contain uppercase");
}
```

### 2.2 Deterministic Attribute Order

```rust
#[test]
fn data_attribute_order_is_deterministic() {
    let props = select::Props::default();
    let (state, ctx) = select::Machine::init(&props);

    // Render twice and compare attribute order
    let attrs1 = select::Machine::connect(&state, &ctx, &props, &|_| {}).root_attrs();
    let attrs2 = select::Machine::connect(&state, &ctx, &props, &|_| {}).root_attrs();

    let keys1: Vec<_> = attrs1.keys().collect();
    let keys2: Vec<_> = attrs2.keys().collect();
    assert_eq!(keys1, keys2, "attribute order must be deterministic across renders");
}
```

### 2.3 All Data Attributes Use Kebab-Case

```rust
#[test]
fn all_data_ars_attributes_kebab_case() {
    // Test every component's connect output for naming convention compliance
    let components: Vec<Box<dyn Fn() -> Vec<(HtmlAttr, String)>>> = vec![
        Box::new(|| collect_data_attrs::<select::Machine>(select::Props { id: "test-select".into(), ..Default::default() })),
        Box::new(|| collect_data_attrs::<dialog::Machine>(dialog::Props { id: "test-dialog".into(), ..Default::default() })),
        Box::new(|| collect_data_attrs::<checkbox::Machine>(test_checkbox_props())),
        Box::new(|| collect_data_attrs::<tabs::Machine>(tabs::Props { id: "test-tabs".into(), ..Default::default() })),
        // ... all components — each must provide a valid `id` via test fixture props
    ];

    for get_attrs in &components {
        for (key, _value) in get_attrs() {
            if let HtmlAttr::Data(s) = key {
                if s.starts_with("ars-") {
                    assert!(
                        !s.contains('_'),
                        "data attribute {s} must not contain underscores"
                    );
                    assert!(
                        s == &s.to_lowercase(),
                        "data attribute {s} must be lowercase"
                    );
                    assert!(
                        s.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                        "data attribute {s} must use only lowercase ASCII and hyphens"
                    );
                }
            }
        }
    }
}

/// Collect data attributes from a machine's connect output.
/// Instead of relying on `M::Props: Default` (which may not provide a valid `id`
/// via `HasId`), use a fixture function that returns test-appropriate props.
fn collect_data_attrs<M: Machine>(props: M::Props) -> Vec<(HtmlAttr, String)> {
    let (state, ctx) = M::init(&props);
    let api = M::connect(&state, &ctx, &props, &|_: M::Event| {});
    api.root_attrs()
        .iter()
        .filter(|(k, _)| matches!(k, HtmlAttr::Data(_)))
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect()
}

// Example fixture: test props must provide a valid `id` via HasId.
fn test_checkbox_props() -> checkbox::Props {
    checkbox::Props::new("test-checkbox-1")
}
```

### 2.4 Snapshot Test for Attribute Stability

```rust
#[test]
fn data_attribute_snapshot_select() {
    let props = select::Props::default();
    let (state, ctx) = select::Machine::init(&props);
    let api = select::Machine::connect(&state, &ctx, &props, &|_| {});

    let data_attrs: BTreeMap<String, String> = api.root_attrs()
        .iter()
        // NOTE: This filter mirrors collect_data_attrs (§2.3) but is intentionally separate —
        // this test verifies ars-prefixed data attrs specifically for CSS selector stability.
        .filter(|(k, _)| matches!(k, HtmlAttr::Data(s) if s.starts_with("ars-")))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Snapshot: if this changes, it's a breaking change for CSS selectors
    insta::assert_yaml_snapshot!("select_data_attrs_closed", data_attrs);
}

#[test]
fn data_attribute_snapshot_dialog_open() {
    let props = dialog::Props::default();
    let mut svc = Service::<dialog::Machine>::new(props);
    svc.send(dialog::Event::Open);
    let api = dialog::Machine::connect(svc.state(), svc.context(), svc.props(), &|_: dialog::Event| {});

    let data_attrs: BTreeMap<String, String> = api.root_attrs()
        .iter()
        .filter(|(k, _)| matches!(k, HtmlAttr::Data(s) if s.starts_with("ars-")))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    insta::assert_yaml_snapshot!("dialog_data_attrs_open", data_attrs);
}
```
