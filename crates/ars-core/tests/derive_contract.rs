//! Derive-contract coverage for `HasId` and `ComponentPart`.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use ars_core::{ComponentPart, HasId};

#[derive(HasId)]
struct Props {
    pub id: String,
    pub label: String,
}

#[derive(HasId)]
struct GenericProps<T>
where
    T: Clone + PartialEq,
{
    pub id: String,
    pub value: T,
}

#[derive(HasId)]
struct LifetimeProps<'a, T>
where
    T: ?Sized,
{
    pub id: String,
    pub value: &'a T,
}

#[derive(ComponentPart)]
#[scope = "dialog"]
enum DialogPart {
    Root,
    Trigger,
    CloseTrigger,
}

#[derive(ComponentPart)]
#[scope = "tabs"]
enum TabsPart {
    Root,
    List,
    Tab(String, String),
    Panel { panel_id: String, tab_id: String },
    TabIndicator,
}

#[derive(ComponentPart)]
#[scope = "generic"]
enum GenericPart<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Eq + Hash + Default + 'static,
{
    Root,
    HiddenInput,
    Item(T),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct CustomDefaultKey(&'static str);

#[derive(ComponentPart)]
#[scope = "named"]
enum NamedPart {
    Root,
    FieldRow { key: CustomDefaultKey, index: usize },
    HelperText { field_id: String },
}

#[derive(ComponentPart)]
#[scope = "naming"]
enum NamingPart {
    Root,
    URLField,
    HTMLInput,
    Step2Label,
}

#[derive(ComponentPart)]
#[scope = "ordered"]
enum OrderedPart {
    Root,
    Gamma,
    Alpha,
    Beta(String),
    Delta { id: String },
}

#[test]
fn has_id_derive_exposes_spec_contract() {
    let mut props = Props {
        id: String::from("heading-1"),
        label: String::from("Title"),
    };
    assert_eq!(props.id(), "heading-1");

    props.set_id(String::from("heading-3"));
    assert_eq!(props.id(), "heading-3");

    let renamed = props.with_id(String::from("heading-2"));
    assert_eq!(renamed.id(), "heading-2");
    assert_eq!(renamed.label, "Title");
}

#[test]
fn has_id_derive_preserves_generics_and_where_clauses() {
    let props = GenericProps {
        id: String::from("generic-1"),
        value: vec![1_u8, 2, 3],
    };
    assert_eq!(props.id(), "generic-1");

    let renamed = props.with_id(String::from("generic-2"));
    assert_eq!(renamed.id(), "generic-2");
    assert_eq!(renamed.value, vec![1_u8, 2, 3]);
}

#[test]
fn has_id_derive_supports_lifetimes() {
    let value = String::from("payload");
    let props = LifetimeProps {
        id: String::from("lifetime-1"),
        value: &value,
    };
    assert_eq!(props.id(), "lifetime-1");

    let renamed = props.with_id(String::from("lifetime-2"));
    assert_eq!(renamed.id(), "lifetime-2");
    assert_eq!(renamed.value, "payload");
}

#[test]
fn component_part_derive_supports_unit_variants() {
    assert_eq!(DialogPart::ROOT, DialogPart::Root);
    assert_eq!(DialogPart::scope(), "dialog");
    assert_eq!(DialogPart::CloseTrigger.name(), "close-trigger");
    assert_eq!(
        DialogPart::CloseTrigger.data_attrs(),
        [
            (ars_core::HtmlAttr::Data("ars-scope"), "dialog"),
            (ars_core::HtmlAttr::Data("ars-part"), "close-trigger"),
        ]
    );
    assert_eq!(
        DialogPart::all(),
        vec![
            DialogPart::Root,
            DialogPart::Trigger,
            DialogPart::CloseTrigger
        ]
    );
}

#[test]
fn component_part_derive_supports_mixed_variants_and_trait_impls() {
    assert_eq!(TabsPart::ROOT, TabsPart::Root);
    assert_eq!(TabsPart::scope(), "tabs");
    assert_eq!(TabsPart::Tab(String::new(), String::new()).name(), "tab");
    assert_eq!(
        TabsPart::Panel {
            panel_id: String::from("panel-1"),
            tab_id: String::from("tab-1"),
        }
        .name(),
        "panel"
    );
    assert_eq!(
        TabsPart::all(),
        vec![
            TabsPart::Root,
            TabsPart::List,
            TabsPart::Tab(String::new(), String::new()),
            TabsPart::Panel {
                panel_id: String::new(),
                tab_id: String::new(),
            },
            TabsPart::TabIndicator,
        ]
    );

    let part = TabsPart::Panel {
        panel_id: String::from("panel-1"),
        tab_id: String::from("tab-1"),
    };
    let clone = part.clone();
    assert_eq!(part, clone);
    assert!(format!("{clone:?}").contains("Panel"));

    let mut left = DefaultHasher::new();
    let mut right = DefaultHasher::new();
    part.hash(&mut left);
    clone.hash(&mut right);
    assert_eq!(left.finish(), right.finish());
}

#[test]
fn component_part_derive_preserves_generics_where_clauses_and_kebab_case() {
    assert_eq!(GenericPart::<String>::ROOT, GenericPart::Root);
    assert_eq!(GenericPart::<String>::scope(), "generic");
    assert_eq!(GenericPart::<String>::HiddenInput.name(), "hidden-input");
    assert_eq!(GenericPart::<String>::Item(String::new()).name(), "item");
    assert_eq!(
        GenericPart::<String>::HiddenInput.data_attrs(),
        [
            (ars_core::HtmlAttr::Data("ars-scope"), "generic"),
            (ars_core::HtmlAttr::Data("ars-part"), "hidden-input"),
        ]
    );
    assert_eq!(
        GenericPart::<String>::all(),
        vec![
            GenericPart::Root,
            GenericPart::HiddenInput,
            GenericPart::Item(String::new()),
        ]
    );
}

#[test]
fn component_part_derive_supports_named_payload_variants() {
    assert_eq!(
        NamedPart::FieldRow {
            key: CustomDefaultKey("field"),
            index: 2,
        }
        .name(),
        "field-row"
    );
    assert_eq!(
        NamedPart::HelperText {
            field_id: String::from("field-1"),
        }
        .name(),
        "helper-text"
    );
    assert_eq!(
        NamedPart::all(),
        vec![
            NamedPart::Root,
            NamedPart::FieldRow {
                key: CustomDefaultKey::default(),
                index: usize::default(),
            },
            NamedPart::HelperText {
                field_id: String::default(),
            },
        ]
    );
}

#[test]
fn component_part_derive_handles_acronyms_and_digits_in_kebab_case() {
    assert_eq!(NamingPart::URLField.name(), "url-field");
    assert_eq!(NamingPart::HTMLInput.name(), "html-input");
    assert_eq!(NamingPart::Step2Label.name(), "step2-label");
}

#[test]
fn component_part_derive_preserves_declaration_order_in_all() {
    assert_eq!(
        OrderedPart::all(),
        vec![
            OrderedPart::Root,
            OrderedPart::Gamma,
            OrderedPart::Alpha,
            OrderedPart::Beta(String::new()),
            OrderedPart::Delta { id: String::new() },
        ]
    );
}

#[test]
fn derive_contract_ui_tests() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
