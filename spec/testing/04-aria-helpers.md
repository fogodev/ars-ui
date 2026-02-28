# ARIA Assertion Helpers

Reusable helper functions for validating ARIA contracts. These live in `ars-core/src/test_helpers.rs` (or a dedicated `ars-test-utils` crate).

```rust
use crate::AttrMap;

/// Assert the AttrMap contains the expected `role` attribute.
pub fn assert_role(attrs: &AttrMap, expected: &str) {
    let role = attrs.get(&HtmlAttr::Role)
        .unwrap_or_else(|| panic!("expected role=\"{}\" but no role attribute found", expected));
    assert_eq!(role, expected, "wrong role");
}

/// Assert aria-label matches expected value.
pub fn assert_aria_label(attrs: &AttrMap, expected: &str) {
    let label = attrs.get(&HtmlAttr::Aria(AriaAttr::Label))
        .unwrap_or_else(|| panic!("expected aria-label=\"{}\" but not found", expected));
    assert_eq!(label, expected, "wrong aria-label");
}

/// Assert aria-expanded is present and matches the expected boolean string.
pub fn assert_aria_expanded(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded))
        .unwrap_or_else(|| panic!("expected aria-expanded but not found"));
    assert_eq!(val, if expected { "true" } else { "false" });
}

/// Assert aria-selected is present and matches the expected boolean string.
pub fn assert_aria_selected(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Selected))
        .unwrap_or_else(|| panic!("expected aria-selected but not found"));
    assert_eq!(val, if expected { "true" } else { "false" });
}

/// Assert aria-disabled matches expected boolean.
/// When `expected` is true, `aria-disabled="true"` must be present.
/// When `expected` is false, `aria-disabled` must be absent or `"false"`.
pub fn assert_aria_disabled(attrs: &AttrMap, expected: bool) {
    if expected {
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true"),
            "expected aria-disabled=\"true\""
        );
    } else {
        let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled));
        assert!(
            val.is_none() || val == Some("false"),
            "expected aria-disabled to be absent or \"false\", got {:?}", val
        );
    }
}

/// Assert aria-busy matches expected boolean.
/// When `expected` is true, `aria-busy="true"` must be present.
/// When `expected` is false, `aria-busy` must be absent or `"false"`.
pub fn assert_aria_busy(attrs: &AttrMap, expected: bool) {
    if expected {
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)),
            Some("true"),
            "expected aria-busy=\"true\""
        );
    } else {
        let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Busy));
        assert!(
            val.is_none() || val == Some("false"),
            "expected aria-busy to be absent or \"false\", got {:?}", val
        );
    }
}

/// Assert aria-checked matches expected value (e.g. "true", "false", "mixed").
pub fn assert_aria_checked(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Checked))
        .unwrap_or_else(|| panic!("expected aria-checked=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-checked");
}

/// Assert aria-controls matches expected value.
pub fn assert_aria_controls(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Controls))
        .unwrap_or_else(|| panic!("expected aria-controls=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-controls");
}

/// Assert aria-labelledby matches expected value.
pub fn assert_aria_labelledby(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy))
        .unwrap_or_else(|| panic!("expected aria-labelledby=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-labelledby");
}

/// Assert aria-describedby matches expected value.
pub fn assert_aria_describedby(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
        .unwrap_or_else(|| panic!("expected aria-describedby=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-describedby");
}

/// Assert aria-haspopup matches expected value (e.g. "true", "menu", "listbox", "dialog").
pub fn assert_aria_haspopup(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::HasPopup))
        .unwrap_or_else(|| panic!("expected aria-haspopup=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-haspopup");
}

/// Assert aria-pressed matches expected value (e.g. "true", "false", "mixed").
pub fn assert_aria_pressed(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed))
        .unwrap_or_else(|| panic!("expected aria-pressed=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-pressed");
}

/// Assert aria-activedescendant matches expected value.
pub fn assert_aria_activedescendant(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant))
        .unwrap_or_else(|| panic!("expected aria-activedescendant=\"{}\" but not found", expected));
    assert_eq!(val, expected, "wrong aria-activedescendant");
}

/// Assert data-ars-state matches the expected value.
pub fn assert_data_state(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Data("ars-state"))
        .unwrap_or_else(|| panic!("expected data-ars-state but not found"));
    assert_eq!(val, expected);
}

/// Assert tabindex value.
pub fn assert_tabindex(attrs: &AttrMap, expected: i32) {
    let expected_str = expected.to_string();
    let val = attrs.get(&HtmlAttr::TabIndex)
        .unwrap_or_else(|| panic!("expected tabindex but not found"));
    assert_eq!(
        val, expected_str.as_str(),
        "expected tabindex=\"{expected}\"",
    );
}

pub fn assert_aria_orientation(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
        Some(expected),
        "expected aria-orientation=\"{expected}\"",
    );
}

pub fn assert_aria_valuemin(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)),
        Some(expected),
        "expected aria-valuemin=\"{expected}\"",
    );
}

pub fn assert_aria_valuemax(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
        Some(expected),
        "expected aria-valuemax=\"{expected}\"",
    );
}

pub fn assert_aria_valuenow(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some(expected),
        "expected aria-valuenow=\"{expected}\"",
    );
}

pub fn assert_aria_valuetext(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
        Some(expected),
        "expected aria-valuetext=\"{expected}\"",
    );
}

pub fn assert_aria_multiselectable(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::MultiSelectable));
    if expected {
        assert_eq!(val, Some("true"), "expected aria-multiselectable=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

pub fn assert_aria_required(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Required));
    if expected {
        assert_eq!(val, Some("true"), "expected aria-required=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

pub fn assert_aria_invalid(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid));
    if expected {
        assert_eq!(val, Some("true"), "expected aria-invalid=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

pub fn assert_aria_live(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::Live)),
        Some(expected),
        "expected aria-live=\"{expected}\"",
    );
}

pub fn assert_aria_atomic(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic));
    if expected {
        assert_eq!(val, Some("true"), "expected aria-atomic=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

pub fn assert_aria_owns(attrs: &AttrMap, expected: &str) {
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::Owns)),
        Some(expected),
        "expected aria-owns=\"{expected}\"",
    );
}

pub fn assert_aria_hidden(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden));
    if expected {
        assert_eq!(val, Some("true"), "expected aria-hidden=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

pub fn assert_aria_modal(attrs: &AttrMap, expected: bool) {
    if expected {
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Modal)),
            Some("true"),
            "expected aria-modal=\"true\""
        );
    } else {
        let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Modal));
        assert!(
            val.is_none() || val == Some("false"),
            "expected aria-modal to be absent or \"false\", got {:?}", val
        );
    }
}

pub fn assert_aria_autocomplete(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::AutoComplete)).expect("aria-autocomplete must be present");
    assert_eq!(val, expected, "expected aria-autocomplete={expected}, got {val}");
}

pub fn assert_aria_errormessage(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)).expect("aria-errormessage must be present");
    assert_eq!(val, expected, "expected aria-errormessage={expected}, got {val}");
}

pub fn assert_aria_setsize(attrs: &AttrMap, expected: i32) {
    let val: i32 = attrs.get(&HtmlAttr::Aria(AriaAttr::SetSize)).expect("aria-setsize must be present")
        .parse().expect("aria-setsize must be a valid integer");
    assert_eq!(val, expected, "expected aria-setsize={expected}, got {val}");
}

pub fn assert_aria_posinset(attrs: &AttrMap, expected: u32) {
    let val: u32 = attrs.get(&HtmlAttr::Aria(AriaAttr::PosInSet)).expect("aria-posinset must be present")
        .parse().expect("aria-posinset must be a valid integer");
    assert_eq!(val, expected, "expected aria-posinset={expected}, got {val}");
}

pub fn assert_aria_level(attrs: &AttrMap, expected: u32) {
    let val: u32 = attrs.get(&HtmlAttr::Aria(AriaAttr::Level)).expect("aria-level must be present")
        .parse().expect("aria-level must be a valid integer");
    assert_eq!(val, expected, "expected aria-level={expected}, got {val}");
}

pub fn assert_aria_roledescription(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::RoleDescription)).expect("aria-roledescription must be present");
    assert_eq!(val, expected, "expected aria-roledescription={expected}, got {val}");
}

pub fn assert_aria_current(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Current))
        .expect("aria-current must be present");
    assert_eq!(
        val, expected,
        "expected aria-current=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-rowindex` on a grid/table cell.
pub fn assert_aria_rowindex(attrs: &AttrMap, expected: u32) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::RowIndex))
        .expect("aria-rowindex must be present");
    assert_eq!(
        val,
        &expected.to_string(),
        "expected aria-rowindex=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-colindex` on a grid/table cell.
pub fn assert_aria_colindex(attrs: &AttrMap, expected: u32) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::ColIndex))
        .expect("aria-colindex must be present");
    assert_eq!(
        val,
        &expected.to_string(),
        "expected aria-colindex=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-rowcount` on a grid/table root.
pub fn assert_aria_rowcount(attrs: &AttrMap, expected: i32) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::RowCount))
        .expect("aria-rowcount must be present");
    assert_eq!(
        val,
        &expected.to_string(),
        "expected aria-rowcount=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-colcount` on a grid/table root.
pub fn assert_aria_colcount(attrs: &AttrMap, expected: i32) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::ColCount))
        .expect("aria-colcount must be present");
    assert_eq!(
        val,
        &expected.to_string(),
        "expected aria-colcount=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-sort` on a table column header.
pub fn assert_aria_sort(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Sort))
        .expect("aria-sort must be present");
    assert_eq!(
        val, expected,
        "expected aria-sort=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-readonly` on an input component.
pub fn assert_aria_readonly(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly));
    if expected {
        assert_eq!(
            val,
            Some("true"),
            "expected aria-readonly=\"true\", but got {:?}",
            val
        );
    } else {
        match val {
            None => {} // absent means not readonly
            Some("false") => {} // explicit false is acceptable
            Some(other) => panic!(
                "expected aria-readonly to be absent or \"false\", got \"{}\"",
                other
            ),
        }
    }
}
```

## 1. Usage in component tests

```rust
#[test]
fn accordion_trigger_aria_contract() {
    let props = accordion::Props::default();
    let ctx = accordion::Context {
        value: Bindable::uncontrolled(BTreeSet::from([Key::from("p1")])),
        ..Default::default()
    };
    let state = accordion::State::Idle;
    let api = accordion::Machine::connect(&state, &ctx, &props, &|_: accordion::Event| {});
    let trigger_attrs = api.part_attrs(accordion::Part::ItemTrigger("p1".into(), "p1-content".into()));

    assert_role(&trigger_attrs, "button");
    assert_aria_expanded(&trigger_attrs, true);
    assert_tabindex(&trigger_attrs, 0);
}
```
