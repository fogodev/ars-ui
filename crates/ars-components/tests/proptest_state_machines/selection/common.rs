use std::collections::BTreeSet;

use ars_collections::{CollectionBuilder, Key};
use ars_components::selection::{
    autocomplete, combobox, listbox, menu, menu_bar, segment_group, select,
};
use proptest::prelude::*;

pub(crate) fn key(value: &'static str) -> Key {
    Key::str(value)
}

pub(crate) fn arb_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(key("alpha")),
        Just(key("bravo")),
        Just(key("charlie")),
        Just(key("delta")),
    ]
}

pub(crate) fn arb_disabled_keys() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::vec(arb_key(), 0..3).prop_map(|keys| keys.into_iter().collect())
}

pub(crate) fn segment_group_items(disabled: &BTreeSet<Key>) -> Vec<segment_group::Segment> {
    [key("alpha"), key("bravo"), key("charlie"), key("delta")]
        .into_iter()
        .map(|value| {
            let is_disabled = disabled.contains(&value);

            segment_group::Segment::new(value).disabled(is_disabled)
        })
        .collect()
}

pub(crate) fn listbox_collection() -> ars_collections::StaticCollection<listbox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            listbox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            listbox::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            listbox::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            listbox::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

pub(crate) fn select_collection() -> ars_collections::StaticCollection<select::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            select::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            select::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            select::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            select::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

pub(crate) fn combobox_collection() -> ars_collections::StaticCollection<combobox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            combobox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            combobox::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            combobox::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            combobox::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

pub(crate) fn alternate_combobox_collection() -> ars_collections::StaticCollection<combobox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            combobox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("echo"),
            "Echo",
            combobox::Item {
                label: "Echo".into(),
            },
        )
        .build()
}

pub(crate) fn autocomplete_collection() -> ars_collections::StaticCollection<autocomplete::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            autocomplete::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            autocomplete::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            autocomplete::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            autocomplete::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

pub(crate) fn menu_collection() -> ars_collections::StaticCollection<menu::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            menu::Item {
                label: "Alpha".into(),
                item_type: menu::ItemType::Normal,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: None,
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            menu::Item {
                label: "Bravo".into(),
                item_type: menu::ItemType::Checkbox,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: Some(false),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            menu::Item {
                label: "Charlie".into(),
                item_type: menu::ItemType::Radio {
                    group: "density".into(),
                },
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: Some(false),
            },
        )
        .item(
            key("delta"),
            "Delta",
            menu::Item {
                label: "Delta".into(),
                item_type: menu::ItemType::Submenu,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: None,
            },
        )
        .build()
}

pub(crate) fn menu_bar_collection() -> ars_collections::StaticCollection<menu_bar::Menu> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            menu_bar::Menu {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            menu_bar::Menu {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            menu_bar::Menu {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            menu_bar::Menu {
                label: "Delta".into(),
            },
        )
        .build()
}
