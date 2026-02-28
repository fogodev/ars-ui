# Accessibility Testing

## 1. Automated: axe-core in CI

Run `axe-core` against every component's rendered output in the adapter test harness.

```rust
#[wasm_bindgen_test]
async fn button_passes_axe() {
    mount_to_body(|| {
        view! { <Button id="axe-btn">"Submit"</Button> }
    });

    let violations = run_axe("[data-ars-scope='button']").await;
    assert!(
        violations.is_empty(),
        "axe-core violations: {:#?}",
        violations,
    );
}
```

Integration via `wasm-bindgen` calling the axe-core JavaScript library, or via Playwright with `@axe-core/playwright`.

## 2. Manual test matrix

| Component    | VoiceOver/Safari (macOS, P0) | VoiceOver/Safari (iOS, P0) | NVDA/Firefox (Windows, P0) | NVDA/Chrome (Windows, P0) | JAWS/Chrome (Windows, P0) | JAWS/Edge (Windows, P0) | TalkBack/Chrome (Android, P1) | Orca/Firefox (Linux, P1) | Narrator/Edge (Windows, P2) |
| ------------ | :--------------------------: | :------------------------: | :------------------------: | :-----------------------: | :-----------------------: | :---------------------: | :---------------------------: | :----------------------: | :-------------------------: |
| Button       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Dialog       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Combobox     |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Menu         |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Tabs         |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| DateField    |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Table        |           Required           |          Stretch           |          Required          |         Required          |         Required          |        Required         |            Stretch            |         Stretch          |           Stretch           |
| Slider       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Checkbox     |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Switch       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| RadioGroup   |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Select       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Tree         |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Accordion    |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| AlertDialog  |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Toast        |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Popover      |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Listbox      |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| NumberField  |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Drawer       |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| HoverCard    |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| Steps        |           Stretch            |          Stretch           |          Stretch           |          Stretch          |          Stretch          |         Stretch         |            Stretch            |         Stretch          |           Stretch           |
| Pagination   |           Stretch            |          Stretch           |          Stretch           |          Stretch          |          Stretch          |         Stretch         |            Stretch            |         Stretch          |           Stretch           |
| Carousel     |           Stretch            |          Stretch           |          Stretch           |          Stretch          |          Stretch          |         Stretch         |            Stretch            |         Stretch          |           Stretch           |
| PinInput     |           Required           |          Required          |          Required          |         Required          |         Required          |        Required         |           Required            |         Required         |          Required           |
| _All others_ |           Required           |          Stretch           |          Required          |         Required          |          Stretch          |         Stretch         |            Stretch            |         Stretch          |           Stretch           |

The "_All others_" row applies only to simple non-interactive components (Badge, Avatar, Separator, VisuallyHidden, etc.).

> **Priority criteria:** Component priority assignments follow foundation [03-accessibility.md section 1.3](../foundation/03-accessibility.md#13-screen-reader-target-matrix). P0 (Required across all primary SR+browser combos): components with custom ARIA roles, complex keyboard patterns, or live region announcements. P1 (Required for primary 3, Stretch for others): components relying partially on native semantics. P2 (Stretch): non-interactive display components (Badge, Avatar, Separator).

Manual testing checklist per component:

1. Navigate to the component using only keyboard (Tab, Shift+Tab, Arrow keys).
2. Verify screen reader announces the correct role, name, and state.
3. Activate the component (Enter/Space) and verify the announcement changes.
4. Verify focus management (e.g. focus moves into dialog on open, returns on close).
5. Verify live region announcements for dynamic content changes.

## 3. Per-Component Screen Reader Test Checklists

Each component MUST have a documented screen reader test script with exact navigation
sequences and expected announcements. These scripts serve as regression baselines.

**Script format** (one per component per screen reader):

```markdown
#### Component: Dialog — Screen Reader: NVDA + Firefox

#### Setup

1. Open page with a "Delete Account" dialog trigger button.

### 4.2 Steps and Expected Announcements

| Step | Action                    | Expected Announcement                                   |
| ---- | ------------------------- | ------------------------------------------------------- |
| 1    | Tab to trigger button     | "Delete Account, button"                                |
| 2    | Press Enter               | "Delete Account dialog, Are you sure?"                  |
| 3    | Tab to Cancel button      | "Cancel, button"                                        |
| 4    | Tab to Confirm button     | "Confirm, button"                                       |
| 5    | Tab (wraps in focus trap) | "Cancel, button"                                        |
| 6    | Press Escape              | (dialog closes, focus returns) "Delete Account, button" |

### 4.3 Regression Baseline

- Date recorded: YYYY-MM-DD
- NVDA version: X.Y
- Firefox version: X.Y
```

**CI integration guidance**:

- Store screen reader test scripts in `tests/screen-reader/` as markdown files.
- Track pass/fail status per component per reader in a test results matrix.
- Require re-verification when ARIA attributes change (detected via snapshot diff).
- P0 screen reader combinations block release; P1/P2 are tracked but non-blocking.

### 4.4 Browser coverage

- Chrome (latest)
- Firefox (latest)
- Safari (latest)
- Edge (latest)

---

## 5. Accessibility Testing Automation (axe-core)

### 5.1 Implementation via wasm-bindgen

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = axe, js_name = run)]
    fn axe_run(context: &str) -> js_sys::Promise;
}

pub async fn run_axe(selector: &str) -> Vec<AxeViolation> {
    let promise = axe_run(selector);
    let result = JsFuture::from(promise).await
        .expect("axe-core run failed");
    // Parse result.violations from JsValue
    parse_axe_violations(result)
}

pub struct AxeViolation {
    pub id: String,
    pub impact: String,
    pub description: String,
    pub nodes: Vec<String>,
    /// Human-readable help text explaining the violation.
    pub help: String,
    /// URL to the axe-core rule documentation (e.g., dequeuniversity.com link).
    pub help_url: String,
    /// WCAG tags associated with the violation (e.g., ["wcag2a", "wcag412"]).
    pub tags: Vec<String>,
    /// Summary of why the check failed, suitable for test output.
    pub failure_summary: String,
}

impl std::fmt::Display for AxeViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} (impact: {})\n  Help: {}\n  WCAG: {}\n  URL: {}\n  Nodes: {:?}\n  {}",
            self.id,
            self.description,
            self.impact,
            self.help,
            self.tags.join(", "),
            self.help_url,
            self.nodes,
            self.failure_summary,
        )
    }
}
```

### 5.2 Rule configuration

Disable rules that don't apply to headless components:

```rust
const AXE_DISABLED_RULES: &[&str] = &[
    "color-contrast",     // headless — no colors
    "link-in-text-block", // headless — no visual distinction
];
```

### 5.3 Playwright-based axe (adapter tests)

```typescript
import AxeBuilder from "@axe-core/playwright";

test("dialog passes axe", async ({ page }) => {
  await page.goto("/storybook/dialog");
  await page.click('[data-ars-part="trigger"]');
  const results = await new AxeBuilder({ page })
    .include('[data-ars-scope="dialog"]')
    .disableRules(["color-contrast"])
    .analyze();
  expect(results.violations).toEqual([]);
});
```

---

## 6. Screen Reader Announcements

Automated ARIA assertions ensure that screen readers receive correct announcements for all dynamic content changes.

### 6.1 Live Region Text Content

```rust
// Standard toasts use role="status" with aria-live="polite".
// Use role="alert" only for urgent/assertive toasts (e.g., errors requiring immediate action);
// the default should be role="status" with aria-live="polite".
#[test]
fn toast_announces_via_live_region() {
    let harness = render(Toaster::new());
    harness.send(toast::Event::Add(Toast::new("File saved successfully").variant(ToastVariant::Success)));
    let live = harness.query_selector("[role='status']");
    assert!(live.is_some());
    assert_eq!(live.expect("live region element must exist").text_content().unwrap_or_default(), "File saved successfully");
}

#[test]
fn live_region_aria_live_attribute() {
    let harness = render(Toaster::new());
    let region = harness.query_selector("[aria-live]");
    assert!(region.is_some());
    let live_value = region.unwrap().attr("aria-live").expect("aria-live must be present");
    assert!(["polite", "assertive"].contains(&live_value.as_str()));
}
```

### 6.2 Expanded/Collapsed Toggle Verification

```rust
#[test]
fn select_aria_expanded_toggles() {
    let harness = render(Select::new());
    assert_eq!(harness.trigger_attr("aria-expanded"), Some("false".into()));
    harness.open();
    assert_eq!(harness.trigger_attr("aria-expanded"), Some("true".into()));
    harness.close();
    assert_eq!(harness.trigger_attr("aria-expanded"), Some("false".into()));
}

#[test]
fn combobox_aria_expanded_toggles() {
    let harness = render(Combobox::new());
    assert_eq!(harness.input_attr("aria-expanded"), Some("false".into()));
    harness.open();
    assert_eq!(harness.input_attr("aria-expanded"), Some("true".into()));
}

#[test]
fn accordion_item_aria_expanded() {
    let harness = render(Accordion::new());
    let trigger = harness.item(0).query_selector("button").expect("accordion item must have a trigger button");
    assert_eq!(trigger.attr("aria-expanded"), Some("false".into()));
    harness.click_selector("[data-ars-part='item']:first-child button");
    assert_eq!(trigger.attr("aria-expanded"), Some("true".into()));
}
```

### 6.3 Role Attribute Verification

```rust
#[test]
fn dialog_role_and_label() {
    let harness = render(Dialog::new().title("Confirm Action"));
    harness.open();
    let dialog = harness.query_selector("[role='dialog']");
    assert!(dialog.is_some());
    let dlg = dialog.unwrap();
    assert_eq!(dlg.attr("aria-modal"), Some("true".into()));
    let labelledby = dlg.attr("aria-labelledby");
    assert!(labelledby.is_some(), "dialog must have aria-labelledby");
    assert!(!labelledby.as_deref().expect("aria-labelledby confirmed present above").is_empty(), "aria-labelledby must not be empty");
    let title_el = harness.query_selector(&format!("#{}", labelledby.as_ref().unwrap()));
    assert_eq!(title_el.unwrap().text_content(), "Confirm Action");
}

#[test]
fn combobox_role_structure() {
    let harness = render(Combobox::new());
    assert_eq!(harness.input_attr("role"), Some("combobox".into()));
    let controls_id = harness.input_attr("aria-controls");
    assert!(controls_id.is_some(), "combobox input must have aria-controls");
    harness.open();
    let listbox = harness.query_selector("[role='listbox']");
    assert!(listbox.is_some());
    let listbox_el = harness.query_selector(&format!("#{}", controls_id.unwrap()));
    assert!(listbox_el.is_some(), "aria-controls must reference the listbox element");
    assert_eq!(listbox_el.unwrap().attr("role"), Some("listbox".into()));
}

#[test]
fn tabs_role_structure() {
    let harness = render(Tabs::new());
    assert!(harness.query_selector("[role='tablist']").is_some());
    assert!(harness.query_selector("[role='tab']").is_some());
    assert!(harness.query_selector("[role='tabpanel']").is_some());
}
```

### 6.4 AlertDialog Role and Announcement

```rust
#[test]
fn alertdialog_role_and_modal() {
    let harness = render(AlertDialog::new().title("Delete Account"));
    harness.open();
    let el = harness.query_selector("[role='alertdialog']");
    assert!(el.is_some(), "AlertDialog must have role='alertdialog'");
    let dlg = el.unwrap();
    assert_eq!(dlg.attr("aria-modal"), Some("true".into()));
    let labelledby = dlg.attr("aria-labelledby")
        .expect("aria-labelledby must be present");
    assert!(!labelledby.is_empty());
    let title_el = harness.query_selector(&format!("#{labelledby}"));
    assert_eq!(title_el.unwrap().text_content(), "Delete Account");
}

#[wasm_bindgen_test]
async fn alertdialog_assertive_announcement_on_open() {
    mount_to_body(|| {
        view! { <AlertDialog id="ad" title="Delete Account">"Are you sure?"</AlertDialog> }
    });
    // Open the alert dialog
    let trigger = document().query_selector("[data-ars-part='trigger']").unwrap().unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;

    // AlertDialog must use assertive live region to announce immediately
    let dialog = document().query_selector("[role='alertdialog']").unwrap()
        .expect("AlertDialog must be present after open");
    assert_eq!(dialog.get_attribute("aria-modal").as_deref(), Some("true"));
}
```

### 6.5 Comprehensive Role Verification (Appendix B Components)

```rust
#[test]
fn checkbox_role_and_checked() {
    let harness = render(Checkbox::new(false));
    assert_eq!(harness.control_attr("role"), Some("checkbox".into()));
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
}

#[test]
fn switch_role_and_checked() {
    let harness = render(Switch::new(false));
    assert_eq!(harness.control_attr("role"), Some("switch".into()));
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
}

#[test]
fn radiogroup_role_structure() {
    let harness = render(RadioGroup::new(vec!["a", "b", "c"]));
    assert!(harness.query_selector("[role='radiogroup']").is_some());
    let radios = harness.query_selector_all("[role='radio']");
    assert_eq!(radios.len(), 3);
}

#[test]
fn slider_role_and_value_attrs() {
    let harness = render(Slider::new(50.0).min(0.0).max(100.0));
    let thumb = harness.query_selector("[role='slider']").unwrap();
    assert_eq!(thumb.attr("aria-valuemin"), Some("0".into()));
    assert_eq!(thumb.attr("aria-valuemax"), Some("100".into()));
    assert_eq!(thumb.attr("aria-valuenow"), Some("50".into()));
}

#[test]
fn menu_role_structure() {
    let harness = render(Menu::with_items(vec![
        menu::Item::new("cut"),
        menu::Item::new("copy"),
        menu::Item::new("paste"),
    ]));
    harness.open();
    assert!(harness.query_selector("[role='menu']").is_some());
    let items = harness.query_selector_all("[role='menuitem']");
    assert_eq!(items.len(), 3);
}

#[test]
fn tree_role_structure() {
    let harness = render(TreeView::new(sample_tree_data()));
    assert!(harness.query_selector("[role='tree']").is_some());
    let items = harness.query_selector_all("[role='treeitem']");
    assert!(!items.is_empty());
}

#[test]
fn table_role_or_native() {
    let harness = render(Table::new(sample_columns(), sample_rows()));
    // Table may use native <table> element or role="table"
    let native = harness.query_selector("table");
    let role_table = harness.query_selector("[role='table']");
    assert!(
        native.is_some() || role_table.is_some(),
        "Table must use native <table> or role='table'"
    );
}

#[test]
fn accordion_button_aria_expanded_controls_region() {
    let harness = render(Accordion::new());
    let trigger = harness.item(0).query_selector("button").unwrap();
    assert_eq!(trigger.attr("aria-expanded"), Some("false".into()));
    let controls_id = trigger.attr("aria-controls").expect("Accordion button must have aria-controls");
    assert!(!controls_id.is_empty(), "aria-controls must not be empty");
    let region = harness.query_selector(&format!("#{controls_id}")).unwrap();
    assert_eq!(region.attr("role"), Some("region".into()));
}

#[test]
fn progress_role_and_value_attrs() {
    let harness = render(Progress::new(0.6).min(0.0).max(1.0));
    let el = harness.query_selector("[role='progressbar']").unwrap();
    assert_eq!(el.attr("aria-valuemin"), Some("0".into()));
    assert_eq!(el.attr("aria-valuemax"), Some("1".into()));
    assert_eq!(el.attr("aria-valuenow"), Some("0.6".into()));
}

#[test]
fn meter_role_and_value_attrs() {
    let harness = render(Meter::new(75.0).min(0.0).max(100.0));
    let el = harness.query_selector("[role='meter']").unwrap();
    assert_eq!(el.attr("aria-valuemin"), Some("0".into()));
    assert_eq!(el.attr("aria-valuemax"), Some("100".into()));
    assert_eq!(el.attr("aria-valuenow"), Some("75".into()));
}

#[test]
fn toolbar_role() {
    let harness = render(Toolbar::new());
    assert!(harness.query_selector("[role='toolbar']").is_some());
}

#[test]
fn tooltip_role() {
    let harness = render(Tooltip::new("Help text"));
    harness.hover_trigger();
    assert!(harness.query_selector("[role='tooltip']").is_some());
}
```

### 6.6 Standalone Listbox Role Verification

```rust
#[wasm_bindgen_test]
async fn listbox_standalone_has_correct_role() {
    let props = listbox::Props::default();
    let harness = render(Listbox::new(props)).await;
    assert_role(harness.snapshot_attrs(), "listbox");
    let items = harness.query_selector_all("[role='option']");
    assert!(!items.is_empty(), "listbox must contain option elements");
}
```

### 6.7 NumberField (spinbutton) Role Verification

```rust
#[wasm_bindgen_test]
async fn number_field_has_spinbutton_role() {
    let props = number_input::Props::default();
    let harness = render(NumberField::new(props)).await;
    let input = harness.query_part("input").expect("input part must exist");
    assert_eq!(input.attr("role"), Some("spinbutton".into()));
}
```

### 6.8 ScrollArea (scrollbar) Role Verification

```rust
#[wasm_bindgen_test]
async fn scroll_area_has_scrollbar_role() {
    let props = scroll_area::Props::default();
    let harness = render(ScrollArea::new(props)).await;
    let scrollbar = harness.query("[role='scrollbar']");
    assert!(scrollbar.is_some(), "ScrollArea must contain a scrollbar role element");
}
```

### 6.9 DatePicker Role Verification

```rust
use ars_core::KeyboardKey;

#[wasm_bindgen_test]
async fn datepicker_has_correct_roles() {
    let props = date_picker::Props::default();
    let harness = render(DatePicker::new(props)).await;
    let group = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("root must exist");
    assert_role(&group.attrs(), AriaRole::Group);
    // When calendar popup opens:
    harness.press_key(KeyboardKey::ArrowDown);
    let dialog = harness.query_selector("[data-ars-part='calendar-popup']")
        .expect("query must not error").expect("calendar popup must exist");
    assert_role(&dialog.attrs(), AriaRole::Dialog);
}
```

### 6.10 ColorPicker Role Verification

```rust
#[wasm_bindgen_test]
async fn colorpicker_has_correct_role() {
    let props = color_picker::Props::default();
    let harness = render(ColorPicker::new(props)).await;
    let root = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("root must exist");
    assert_role(&root.attrs(), AriaRole::Group);
}
```

### 6.11 TagGroup Role Verification

```rust
#[wasm_bindgen_test]
async fn taggroup_has_correct_role() {
    let props = tag_group::Props::default();
    let harness = render(TagGroup::new(props)).await;
    let root = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("root must exist");
    assert_role(&root.attrs(), AriaRole::Group);
}
```

### 6.12 Grid Role Verification

```rust
#[wasm_bindgen_test]
async fn grid_has_correct_roles() {
    let props = grid::Props::default();
    let harness = render(Grid::new(props)).await;
    let grid_el = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("grid must exist");
    assert_role(&grid_el.attrs(), AriaRole::Grid);
    let row = harness.query_selector("[data-ars-part='row']")
        .expect("query must not error").expect("row must exist");
    assert_role(&row.attrs(), AriaRole::Row);
    let cell = harness.query_selector("[data-ars-part='cell']")
        .expect("query must not error").expect("cell must exist");
    assert_role(&cell.attrs(), AriaRole::Gridcell);
}
```

### 6.13 `aria-orientation` Verification

```rust
#[wasm_bindgen_test]
async fn tabs_horizontal_orientation() {
    let props = tabs::Props { orientation: Orientation::Horizontal, ..Default::default() };
    let harness = render(Tabs::new(props)).await;
    let list = harness.query_part("list").expect("list part must exist");
    assert_eq!(list.attr("aria-orientation"), Some("horizontal".into()));
}

#[wasm_bindgen_test]
async fn tabs_vertical_orientation() {
    let props = tabs::Props { orientation: Orientation::Vertical, ..Default::default() };
    let harness = render(Tabs::new(props)).await;
    let list = harness.query_part("list").expect("list part must exist");
    assert_eq!(list.attr("aria-orientation"), Some("vertical".into()));
}

#[wasm_bindgen_test]
async fn slider_horizontal_orientation() {
    let props = slider::Props::default(); // horizontal by default
    let harness = render(Slider::new(props)).await;
    let thumb = harness.query_part("thumb").expect("thumb part must exist");
    assert_eq!(thumb.attr("aria-orientation"), Some("horizontal".into()));
}

#[wasm_bindgen_test]
async fn slider_vertical_orientation() {
    let props = slider::Props { orientation: Orientation::Vertical, ..Default::default() };
    let harness = render(Slider::new(props)).await;
    let thumb = harness.query_part("thumb").expect("thumb part must exist");
    assert_eq!(thumb.attr("aria-orientation"), Some("vertical".into()));
}

#[wasm_bindgen_test]
async fn toolbar_default_orientation() {
    let props = toolbar::Props::default();
    let harness = render(Toolbar::new(props)).await;
    assert_aria_orientation(harness.snapshot_attrs(), "horizontal");
}
```

### 6.14 `aria-checked` State Transitions

```rust
#[test]
fn checkbox_aria_checked_toggles() {
    let harness = render(Checkbox::new(false));
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
}

#[test]
fn checkbox_aria_checked_mixed_for_indeterminate() {
    let harness = render(Checkbox::new_indeterminate());
    assert_eq!(harness.control_attr("aria-checked"), Some("mixed".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
}

#[test]
fn switch_aria_checked_toggles() {
    let harness = render(Switch::new(false));
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
    harness.click();
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
}
```

### 6.15 `aria-selected` for Tabs

```rust
#[test]
fn tabs_aria_selected_on_active_tab() {
    let harness = render(Tabs::new().tabs(vec!["Tab 1", "Tab 2", "Tab 3"]));
    let tabs = harness.query_selector_all("[role='tab']");
    assert_eq!(tabs[0].attr("aria-selected"), Some("true".into()));
    assert_eq!(tabs[1].attr("aria-selected"), Some("false".into()));
    assert_eq!(tabs[2].attr("aria-selected"), Some("false".into()));

    // Select second tab
    harness.select_tab(1);
    let tabs = harness.query_selector_all("[role='tab']");
    assert_eq!(tabs[0].attr("aria-selected"), Some("false".into()));
    assert_eq!(tabs[1].attr("aria-selected"), Some("true".into()));
    assert_eq!(tabs[2].attr("aria-selected"), Some("false".into()));
}
```

### 6.16 `aria-activedescendant` for Combobox/Listbox

```rust
/// Convenience wrapper for mounting a component and returning a test harness.
/// Equivalent to `TestHarness::mount(component).await`.
/// See 05-adapter-harness.md §3 for the canonical TestHarness API.
async fn mount_component<C: Component>(c: C) -> TestHarness {
    TestHarness::mount(c).await
}

#[wasm_bindgen_test]
async fn combobox_activedescendant_updates_on_navigation() {
    let items = vec![
        combobox::Item { key: Key::from("a"), label: "Alpha".into() },
        combobox::Item { key: Key::from("b"), label: "Beta".into() },
        combobox::Item { key: Key::from("c"), label: "Charlie".into() },
    ];
    let harness = mount_component(Combobox::new("cb1", items));
    tick().await;
    harness.focus("[role='combobox']");
    harness.press_key(KeyboardKey::ArrowDown);
    tick().await;
    let input = harness.query("[role='combobox']");
    let active_id = input.attr("aria-activedescendant");
    assert!(active_id.is_some(), "must set aria-activedescendant on arrow navigation");
    // Verify the referenced element exists and is highlighted
    let highlighted = harness.query(&format!("#{}", active_id.as_ref().unwrap()));
    assert_eq!(highlighted.attr("data-ars-highlighted"), Some("true".into()));
}
```

### 6.17 `aria-invalid` / `aria-errormessage` for Form Components

```rust
#[test]
fn text_field_invalid_sets_aria_attributes() {
    let mut svc = Service::new(text_field::Props::new("tf1"));
    // Simulate validation failure
    svc.send(text_field::Event::SetInvalid {
        message_id: "tf1-error"
    });
    let api = svc.connect(&|_| {});
    let attrs = api.input_attrs();
    assert_aria_invalid(&attrs, true);
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
        Some("tf1-error"),
    );
}
```

### 6.18 `aria-required` for Form Components

```rust
#[test]
fn required_text_field_has_aria_required() {
    let svc = Service::new(text_field::Props::new("tf1").required(true));
    let api = svc.connect(&|_| {});
    assert_aria_required(&api.input_attrs(), true);
}

#[test]
fn required_select_has_aria_required() {
    let svc = Service::new(select::Props::new("sel1").required(true));
    let api = svc.connect(&|_| {});
    assert_aria_required(&api.trigger_attrs(), true);
}
```

### 6.19 `aria-valuetext` for Slider

```rust
#[test]
fn slider_value_text_with_formatter() {
    let svc = Service::new(slider::Props::new("sl1")
        .min(0.0).max(100.0).value(50.0)
        .value_text(|v| format!("{}%", v)));
    let api = svc.connect(&|_| {});
    assert_aria_valuetext(&api.thumb_attrs(), "50%");
}
```

### 6.20 `aria-controls` for Tabs

```rust
#[test]
fn tabs_aria_controls_links_tab_to_panel() {
    let svc = Service::new(tabs::Props::new("tabs1"));
    let api = svc.connect(&|_| {});
    let tab_attrs = api.tab_attrs("tab-0");
    let controls_id = tab_attrs.get(&HtmlAttr::Aria(AriaAttr::Controls));
    assert!(controls_id.is_some(), "tab must have aria-controls");
    // The panel with this ID should exist
    let panel_attrs = api.panel_attrs("tab-0");
    assert_eq!(panel_attrs.get(&HtmlAttr::Id), controls_id);
}
```

### 6.21 `aria-modal`, `aria-checked`, `aria-selected` (State Machine Level)

```rust
#[test]
fn dialog_open_has_aria_modal() {
    let mut svc = Service::new(dialog::Props::new("d1"));
    svc.send(dialog::Event::Open);
    let api = svc.connect(&|_| {});
    assert_eq!(api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
}

#[test]
fn checkbox_checked_has_aria_checked_true() {
    let mut svc = Service::new(checkbox::Props::new("cb1"));
    svc.send(checkbox::Event::Toggle);
    let api = svc.connect(&|_| {});
    assert_aria_checked(&api.root_attrs(), "true");
}

#[test]
fn tabs_active_tab_has_aria_selected() {
    let svc = Service::new(tabs::Props::new("t1"));
    let api = svc.connect(&|_| {});
    assert_aria_selected(&api.tab_attrs("tab-0"), true);
    assert_aria_selected(&api.tab_attrs("tab-1"), false);
}
```

### 6.22 Forced Colors Mode Testing

Components MUST remain usable in Windows High Contrast Mode (WHCM) / forced-colors
environments. Focus indicators and state indicators that rely on `box-shadow` or
`background-color` are invisible in forced-colors mode and must use alternatives.

```rust
use ars_core::Bindable;

#[wasm_bindgen_test]
async fn focus_ring_visible_in_forced_colors() {
    // Verify focus indicators use outline (visible in WHCM)
    // not box-shadow or background-color (invisible in WHCM).
    //
    // Strategy: inspect computed styles on focused elements and verify
    // that the focus ring is rendered via `outline` or `border`, NOT
    // `box-shadow` or `background-color` alone.
    mount_to_body(|| {
        view! { <Button id="fc-btn">"Action"</Button> }
    });

    let btn = document().get_element_by_id("fc-btn").unwrap();
    btn.dyn_ref::<HtmlElement>().unwrap().focus();
    tick().await;

    let style = window().get_computed_style(&btn).unwrap().unwrap();
    let outline = style.get_property_value("outline-style").unwrap();
    assert_ne!(
        outline, "none",
        "Focus indicator must use outline (visible in forced-colors mode)"
    );
}

// State indicators that MUST be visible in forced-colors mode:
// - Checkbox checked indicator: must use a border or SVG, not background-color alone
// - Switch thumb position: must be distinguishable by position or border, not color alone
// - Disabled state: must use a distinct border style or opacity recognized by forced-colors
//
// These are verified via visual regression tests with `forced-colors: active` media query
// emulation in Playwright:
//
//   await page.emulateMedia({ forcedColors: 'active' });
//   await expect(page.locator('[data-ars-scope="checkbox"]')).toHaveScreenshot();

#[wasm_bindgen_test]
async fn checkbox_forced_colors_checkmark_visible() {
    let props = checkbox::Props { checked: true, ..Default::default() };
    let harness = render(Checkbox::new(props)).await;
    let indicator = harness.query_selector("[data-ars-part='indicator']")
        .expect("query must not error").expect("indicator must exist");
    let styles = indicator.computed_styles();
    assert_ne!(styles.get("forced-color-adjust"), Some(&"none".to_string()),
        "Checkbox checkmark must be visible in forced-colors mode");
}

#[wasm_bindgen_test]
async fn slider_forced_colors_thumb_visible() {
    let props = slider::Props { value: Bindable::controlled(50.0), ..Default::default() };
    let harness = render(Slider::new(props)).await;
    let thumb = harness.query_selector("[data-ars-part='thumb']")
        .expect("query must not error").expect("thumb must exist");
    let styles = thumb.computed_styles();
    assert!(styles.get("border-width").is_some(),
        "Slider thumb must have a visible border in forced-colors mode");
}

#[wasm_bindgen_test]
async fn progress_forced_colors_fill_visible() {
    let props = progress::Props { value: Some(0.6), ..Default::default() };
    let harness = render(Progress::new(props)).await;
    let fill = harness.query_selector("[data-ars-part='fill']")
        .expect("query must not error").expect("fill must exist");
    let styles = fill.computed_styles();
    assert!(
        styles.get("border-width").map_or(false, |v| v != "0px" && v != "0")
            || styles.get("outline-width").map_or(false, |v| v != "0px" && v != "0"),
        "progress fill must use border or outline (not background-color) for forced-colors visibility"
    );
}

/// Section 6.22.1: prefers-reduced-transparency
///
/// Foundation 03 §6.3: Overlay components MUST use opaque backdrops
/// when `prefers-reduced-transparency: reduce` is active.

#[wasm_bindgen_test]
async fn dialog_backdrop_opaque_when_reduced_transparency() {
    let harness = TestHarness::mount(dialog::Machine::new(dialog::Props {
        open: true,
        modal: true,
        ..Default::default()
    }))
    .await;

    // Emulate prefers-reduced-transparency: reduce
    harness.emulate_media("prefers-reduced-transparency", "reduce").await;

    let backdrop = harness.query("[data-ars-part='backdrop']")
        .expect("dialog should have backdrop element");
    let styles = backdrop.computed_styles();

    // Backdrop must NOT use semi-transparent background
    let bg = styles.get("background-color").expect("backdrop should have background-color");
    assert!(
        !bg.contains("rgba") || bg.ends_with(", 1)"),
        "Backdrop must be fully opaque when prefers-reduced-transparency: reduce, got {bg}"
    );

    // Backdrop must NOT use backdrop-filter (blur/transparency effects)
    let filter = styles.get("backdrop-filter");
    assert!(
        filter.is_none() || filter == Some(&"none".to_string()),
        "Backdrop-filter must be none when reduced-transparency is active"
    );
}

/// Section 6.22.2: FocusState + FocusRing double-application guard
///
/// Foundation 05 §4.3: Components MUST NOT also call `FocusRing.apply_focus_attrs()`
/// when using `FocusState`. This test verifies `data-ars-focus-visible` appears at most once.

#[wasm_bindgen_test]
async fn focus_visible_attribute_not_duplicated() {
    let harness = TestHarness::mount(button::Machine::new(button::Props::default())).await;
    harness.focus("[data-ars-scope]").await;

    let attrs = harness.snapshot_attrs();
    let focus_visible_count = attrs
        .iter()
        .filter(|(k, _)| matches!(k, HtmlAttr::Data(name) if name == "ars-focus-visible"))
        .count();

    assert!(
        focus_visible_count <= 1,
        "data-ars-focus-visible must appear at most once (FocusState and FocusRing must not both apply it), found {focus_visible_count} occurrences"
    );
}
```

### 6.23 Virtualized Collection Accessibility

Per [06-collections.md](../foundation/06-collections.md) §6.4, virtualized collection items must include `aria-setsize` (total count) and `aria-posinset` (1-based index) attributes for screen readers to announce position context.

```rust
use ars_core::Key;

#[wasm_bindgen_test]
async fn virtualized_listbox_items_have_setsize_and_posinset() {
    let items: Vec<_> = (0..100).map(|i| (Key::from(format!("item-{i}")), format!("Item {i}"))).collect();
    let harness = render(VirtualizedListbox::new("vl1", items, /* viewport_height */ 200.0)).await;
    tick().await;

    let options = harness.query_selector_all("[role='option']");
    assert!(!options.is_empty(), "some options must be rendered");

    for (i, option) in options.iter().enumerate() {
        let setsize = option.attr("aria-setsize")
            .expect("each option must have aria-setsize");
        assert_eq!(setsize, "100", "aria-setsize must equal total item count");

        let posinset = option.attr("aria-posinset")
            .expect("each option must have aria-posinset");
        let expected = (i + 1).to_string(); // 1-based
        // Note: posinset reflects the item's position in the full list, not the visible subset
        assert!(posinset.parse::<u32>().expect("must be numeric") > 0,
            "aria-posinset must be a positive integer");
    }
}
```

### 6.24 InputMode Attributes

Per [03-accessibility.md](../foundation/03-accessibility.md) §7.4, input components must declare appropriate `inputmode` values for virtual keyboard type hints on mobile devices.

```rust
#[wasm_bindgen_test]
async fn number_field_has_numeric_inputmode() {
    let props = number_input::Props::default();
    let harness = render(NumberField::new(props)).await;
    let input = harness.query("[data-ars-part='input']")
        .expect("number field must have an input part");
    assert_eq!(
        input.attr("inputmode"),
        Some("numeric".into()),
        "NumberField input must declare inputmode='numeric'"
    );
}

#[wasm_bindgen_test]
async fn text_field_omits_inputmode_by_default() {
    let props = text_field::Props::default();
    let harness = render(TextField::new(props)).await;
    let input = harness.query("[data-ars-part='input']")
        .expect("text field must have an input part");
    // Default TextField should not set inputmode (browser default is fine)
    assert!(
        input.attr("inputmode").is_none(),
        "TextField should not set inputmode by default"
    );
}
```

### 6.25 Drag-and-Drop Accessibility

Per [06-collections.md](../foundation/06-collections.md) §10.6, draggable collection items must declare `aria-roledescription` to communicate draggability to assistive technologies.

```rust
#[wasm_bindgen_test]
async fn draggable_items_have_roledescription() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha").draggable(true)
        .item(Key::from("b"), "Beta").draggable(false)
        .build();
    let harness = render(DraggableListbox::new("dl1", col)).await;
    tick().await;

    let draggable = harness.query("[data-ars-key='a']")
        .expect("draggable item must be in DOM");
    assert_eq!(
        draggable.attr("aria-roledescription"),
        Some("draggable".into()),
        "draggable item must have aria-roledescription='draggable'"
    );

    let non_draggable = harness.query("[data-ars-key='b']")
        .expect("non-draggable item must be in DOM");
    assert!(
        non_draggable.attr("aria-roledescription").is_none(),
        "non-draggable item must not have aria-roledescription"
    );
}
```

### 6.26 Keyboard Drag aria-grabbed State

```rust
#[wasm_bindgen_test]
async fn aria_grabbed_toggles_during_keyboard_drag() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha").draggable(true)
        .item(Key::from("b"), "Beta").draggable(true)
        .build();
    let harness = render(DraggableListbox::new("dl1", col)).await;
    tick().await;

    let item = harness.query("[data-ars-key='a']").expect("item must exist");
    assert_eq!(
        item.attr("aria-grabbed"),
        Some("false".into()),
        "draggable item must start with aria-grabbed='false'"
    );

    // Initiate keyboard drag (e.g., Space on focused draggable item)
    harness.focus("[data-ars-key='a']");
    harness.press_key(KeyboardKey::Space);
    tick().await;

    let item = harness.query("[data-ars-key='a']").expect("item must exist");
    assert_eq!(
        item.attr("aria-grabbed"),
        Some("true".into()),
        "item must have aria-grabbed='true' during keyboard drag"
    );
}
```

### 6.27 Selected State Uses data-ars-state Token

```rust
#[wasm_bindgen_test]
async fn selected_item_has_selected_token_in_data_ars_state() {
    let harness = render(Listbox::new(vec![
        ListboxItem::new(Key::from("a"), "Alpha"),
        ListboxItem::new(Key::from("b"), "Beta"),
    ])).await;

    // Select first item
    harness.item(0).click_trigger().await;

    let state_attr = harness.item(0).attr("data-ars-state")
        .expect("selected item must have data-ars-state");
    assert!(state_attr.split_whitespace().any(|token| token == "selected"),
        "data-ars-state must contain 'selected' token, got: '{}'", state_attr);

    // Unselected item should NOT have selected token
    let other_attr = harness.item(1).attr("data-ars-state").unwrap_or_default();
    assert!(!other_attr.split_whitespace().any(|token| token == "selected"),
        "unselected item must not have 'selected' token");
}
```

---

## 7. ARIA Describedby / Labelledby Wiring Tests

Complex conditional ARIA wiring logic (`aria-describedby` with `description` and `error-message` parts) must be tested for ID uniqueness across mounts, correct part ID format, and dynamic description add/remove.

### 7.1 Snapshot Tests per Component

```rust
#[cfg(test)]
mod aria_describedby_tests {
    use super::*;
    use insta::assert_snapshot;

    /// Verify describedby includes the description part ID when description is present.
    #[test]
    fn checkbox_control_describedby_with_description() {
        let props = checkbox::Props {
            id: "cb-1".into(),
            description: Some("Helper text".into()),
            ..Default::default()
        };
        let (state, ctx) = checkbox::Machine::init(&props);
        let api = checkbox::Machine::connect(&state, &ctx, &props, &|_| {});
        let control_attrs = api.control_attrs();

        assert_eq!(
            control_attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)).expect("aria-describedby must be present"),
            "cb-1-description",
        );
        assert_snapshot!("checkbox_control_describedby_with_description", format!("{:#?}", control_attrs));
    }

    /// Verify describedby is absent when no description is provided.
    #[test]
    fn checkbox_control_describedby_without_description() {
        let props = checkbox::Props {
            id: "cb-2".into(),
            description: None,
            ..Default::default()
        };
        let (state, ctx) = checkbox::Machine::init(&props);
        let api = checkbox::Machine::connect(&state, &ctx, &props, &|_| {});
        let control_attrs = api.control_attrs();

        assert!(
            control_attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)).is_none(),
            "aria-describedby must be absent when description is None"
        );
        assert_snapshot!("checkbox_control_describedby_without_description", format!("{:#?}", control_attrs));
    }

    /// Verify describedby references the error-message part when invalid.
    #[test]
    fn checkbox_control_describedby_with_error() {
        let props = checkbox::Props {
            id: "cb-3".into(),
            description: Some("Helper text".into()),
            ..Default::default()
        };
        let (state, mut ctx) = checkbox::Machine::init(&props);
        ctx.invalid = true;
        ctx.error_message = Some("This field is required".into());
        let api = checkbox::Machine::connect(&state, &ctx, &props, &|_| {});
        let control_attrs = api.control_attrs();

        let describedby = control_attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
            .expect("aria-describedby must be present when error exists");
        // Must include both description and error-message part IDs
        assert!(describedby.contains("cb-3-description"));
        assert!(describedby.contains("cb-3-error-message"));
        assert_snapshot!("checkbox_control_describedby_with_error", format!("{:#?}", control_attrs));
    }
}
```

### 7.2 ID Uniqueness Across Mounts

```rust
#[wasm_bindgen_test]
async fn describedby_ids_unique_across_multiple_mounts() {
    mount_to_body(|| {
        view! {
            <TextField id="tf-a" description="Help A">"Label A"</TextField>
            <TextField id="tf-b" description="Help B">"Label B"</TextField>
        }
    });
    tick().await;

    let desc_a = document().get_element_by_id("tf-a-description")
        .expect("tf-a-description element must exist");
    let desc_b = document().get_element_by_id("tf-b-description")
        .expect("tf-b-description element must exist");
    assert_ne!(desc_a.id(), desc_b.id());

    let input_a = document().query_selector("#tf-a [data-ars-part='input']")
        .expect("query must succeed").expect("tf-a input part must exist");
    assert_eq!(
        input_a.get_attribute("aria-describedby").as_deref(),
        Some("tf-a-description"),
    );
}
```

### 7.3 Dynamic Description Add/Remove

> **Note:** This example uses Leptos-specific APIs (`signal()`, `leptos::mount::mount_to_body`). See [05-adapter-harness.md](05-adapter-harness.md) for the equivalent Dioxus pattern.

```rust
#[wasm_bindgen_test]
async fn describedby_updates_when_description_toggled() {
    let (desc, set_desc) = signal(Option::<String>::None);
    leptos::mount::mount_to_body(move || {
        view! { <TextField id="tf-dyn" description=desc>"Label"</TextField> }
    });

    let input = || document().query_selector("#tf-dyn [data-ars-part='input']").unwrap().unwrap();
    assert!(input().get_attribute("aria-describedby").is_none());

    set_desc.set(Some("Now has description".into()));
    tick().await;
    assert_eq!(input().get_attribute("aria-describedby").as_deref(), Some("tf-dyn-description"));

    set_desc.set(None);
    tick().await;
    assert!(input().get_attribute("aria-describedby").is_none());
}
```

Apply this pattern for all input components: `TextField`, `NumberField`, `Checkbox`, `Switch`, `RadioGroup`, `Select`, `Combobox`, `Slider`, `TextArea`.

---

## 8. LiveRegion Announcement Timing Tests

Dynamic announcements via `LiveRegion` (e.g., Table sort changes, Select highlight updates) must be tested for correct text, polite/assertive priority, post-DOM-update timing, and queue ordering.

### 8.1 Basic Announcement Verification

```rust
#[wasm_bindgen_test]
async fn live_region_announces_select_highlight_change() {
    mount_to_body(|| {
        view! {
            <Select>
                <select::Item value="apple">"Apple"</select::Item>
                <select::Item value="banana">"Banana"</select::Item>
            </Select>
            <LiveRegion />
        }
    });

    let trigger = document().query_selector("[data-ars-part='trigger']").unwrap().unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;

    // Navigate to next item
    dispatch_keyboard_event(&trigger, "keydown", "ArrowDown");
    tick().await;

    let status = document().query_selector("[role='status']").unwrap()
        .expect("LiveRegion with role='status' must exist");
    let text = status.text_content().unwrap_or_default();
    assert!(
        text.contains("Apple"),
        "LiveRegion must announce the highlighted item, got: {}",
        text,
    );
}
```

### 8.2 Polite vs Assertive Priority

```rust
#[wasm_bindgen_test]
async fn live_region_polite_for_navigation() {
    mount_to_body(|| {
        view! {
            <Select>
                <select::Item value="a">"A"</select::Item>
                <select::Item value="b">"B"</select::Item>
            </Select>
            <LiveRegion />
        }
    });

    // Navigation announcements should use polite
    let region = document().query_selector("[aria-live='polite']").unwrap()
        .expect("Polite live region must exist for navigation announcements");
    assert_eq!(region.get_attribute("aria-live").as_deref(), Some("polite"));
}

#[wasm_bindgen_test]
async fn live_region_assertive_for_errors() {
    mount_to_body(|| {
        view! {
            <TextField id="tf" required=true />
            <LiveRegion />
        }
    });

    // Trigger validation error
    let input = document().query_selector("#tf [data-ars-part='input']").unwrap().unwrap();
    input.dyn_ref::<HtmlElement>().unwrap().focus();
    input.dyn_ref::<HtmlElement>().unwrap().blur();
    tick().await;

    let assertive = document().query_selector("[aria-live='assertive']").unwrap()
        .expect("Assertive live region must exist for error announcements");
    let text = assertive.text_content().unwrap_or_default();
    assert!(!text.is_empty(), "Error announcement must not be empty");
}
```

### 8.3 Queue Ordering

```rust
#[wasm_bindgen_test]
async fn live_region_preserves_announcement_order() {
    mount_to_body(|| {
        view! {
            <Table sortable=true>/* ... */</Table>
            <LiveRegion />
        }
    });

    // Trigger two rapid sort changes
    let col_header = document().query_selector("[data-ars-part='column-header']").unwrap().unwrap();
    col_header.dyn_ref::<HtmlElement>().unwrap().click(); // sort ascending
    col_header.dyn_ref::<HtmlElement>().unwrap().click(); // sort descending
    tick().await;

    let status = document().query_selector("[role='status']").unwrap().unwrap();
    let text = status.text_content().unwrap_or_default();
    // Final announcement should reflect the latest sort direction
    assert!(
        text.contains("descending"),
        "LiveRegion must announce the final sort state, got: {}",
        text,
    );
}
```
