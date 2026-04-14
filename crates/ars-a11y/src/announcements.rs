//! Localizable screen-reader announcement message helpers.

use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};

use ars_core::{ComponentMessages, Locale, MessageFn};

use crate::aria::attribute::AriaSort;

type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

type CountLocaleMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;

type LabelLocaleMessage = dyn Fn(&str, &Locale) -> String + Send + Sync;

type FieldErrorLocaleMessage = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

type MoveLocaleMessage = dyn Fn(&str, usize, usize, &Locale) -> String + Send + Sync;

/// Localizable announcement templates for common component state changes.
///
/// Announcement helpers follow the shared `ComponentMessages` pattern: each
/// field is a [`MessageFn`] that receives the active locale when the helper is
/// invoked, allowing the adapter to supply locale-sensitive strings and
/// pluralization rules without hardcoding English in the subsystem layer.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Message used when a search yields any number of results.
    ///
    /// The default English implementation branches on `count`, while
    /// locale-specific implementations can use richer pluralization rules.
    pub search_results: MessageFn<CountLocaleMessage>,

    /// Message used when an item becomes selected.
    pub selected: MessageFn<LabelLocaleMessage>,

    /// Message used when an item becomes deselected.
    pub deselected: MessageFn<LabelLocaleMessage>,

    /// Message used when a field has a validation error.
    pub validation_error: MessageFn<FieldErrorLocaleMessage>,

    /// Message used while content is loading.
    pub loading: MessageFn<LocaleMessage>,

    /// Message used when loading finishes.
    pub loading_complete: MessageFn<LocaleMessage>,

    /// Message used when an item is moved to a new position.
    pub item_moved: MessageFn<MoveLocaleMessage>,

    /// Message used when an item is removed.
    pub item_removed: MessageFn<LabelLocaleMessage>,

    /// Message used when a column is sorted ascending.
    pub sorted_ascending: MessageFn<LabelLocaleMessage>,

    /// Message used when a column is sorted descending.
    pub sorted_descending: MessageFn<LabelLocaleMessage>,

    /// Message used when a column is sorted in a non-standard order.
    pub sorted_other: MessageFn<LabelLocaleMessage>,

    /// Message used when a column is not sorted.
    pub not_sorted: MessageFn<LabelLocaleMessage>,

    /// Message used when a tree node expands.
    pub tree_expanded: MessageFn<LabelLocaleMessage>,

    /// Message used when a tree node collapses.
    pub tree_collapsed: MessageFn<LabelLocaleMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            search_results: MessageFn::new(Arc::new(|count: usize, _locale: &Locale| match count {
                0 => String::from("No results found."),
                1 => String::from("1 result found."),
                _ => format!("{count} results found."),
            }) as Arc<CountLocaleMessage>),

            selected: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label}, selected.")
            }) as Arc<LabelLocaleMessage>),

            deselected: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label}, deselected.")
            }) as Arc<LabelLocaleMessage>),

            validation_error: MessageFn::new(Arc::new(
                |field: &str, error: &str, _locale: &Locale| format!("{field}: {error}. Error."),
            ) as Arc<FieldErrorLocaleMessage>),

            loading: MessageFn::static_str("Loading."),

            loading_complete: MessageFn::static_str("Loading complete."),

            item_moved: MessageFn::new(Arc::new(
                |label: &str, position: usize, total: usize, _locale: &Locale| {
                    format!("{label} moved to position {position} of {total}.")
                },
            ) as Arc<MoveLocaleMessage>),

            item_removed: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label} removed.")
            }) as Arc<LabelLocaleMessage>),

            sorted_ascending: MessageFn::new(Arc::new(|column: &str, _locale: &Locale| {
                format!("{column}, sorted ascending.")
            }) as Arc<LabelLocaleMessage>),

            sorted_descending: MessageFn::new(Arc::new(|column: &str, _locale: &Locale| {
                format!("{column}, sorted descending.")
            }) as Arc<LabelLocaleMessage>),

            sorted_other: MessageFn::new(Arc::new(|column: &str, _locale: &Locale| {
                format!("{column}, sorted.")
            }) as Arc<LabelLocaleMessage>),

            not_sorted: MessageFn::new(Arc::new(|column: &str, _locale: &Locale| {
                format!("{column}, not sorted.")
            }) as Arc<LabelLocaleMessage>),

            tree_expanded: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label}, expanded.")
            }) as Arc<LabelLocaleMessage>),

            tree_collapsed: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label}, collapsed.")
            }) as Arc<LabelLocaleMessage>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Helpers that render localized announcement strings for common UI changes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Announcements;

impl Announcements {
    /// Announces the number of search results.
    #[must_use]
    pub fn search_results(count: usize, locale: &Locale, messages: &Messages) -> String {
        (messages.search_results)(count, locale)
    }

    /// Announces a selection change in a collection widget.
    #[must_use]
    pub fn selection_changed(
        label: &str,
        selected: bool,
        locale: &Locale,
        messages: &Messages,
    ) -> String {
        let message = if selected {
            &messages.selected
        } else {
            &messages.deselected
        };

        message(label, locale)
    }

    /// Announces a toast notification.
    #[must_use]
    pub fn toast(message: &str) -> String {
        message.to_string()
    }

    /// Announces a form validation error.
    #[must_use]
    pub fn validation_error(
        field_label: &str,
        error: &str,
        locale: &Locale,
        messages: &Messages,
    ) -> String {
        (messages.validation_error)(field_label, error, locale)
    }

    /// Announces that loading has started.
    #[must_use]
    pub fn loading(locale: &Locale, messages: &Messages) -> String {
        (messages.loading)(locale)
    }

    /// Announces that loading has completed.
    #[must_use]
    pub fn loading_complete(locale: &Locale, messages: &Messages) -> String {
        (messages.loading_complete)(locale)
    }

    /// Announces that an item moved within a collection.
    #[must_use]
    pub fn item_moved(
        label: &str,
        position: usize,
        total: usize,
        locale: &Locale,
        messages: &Messages,
    ) -> String {
        (messages.item_moved)(label, position, total, locale)
    }

    /// Announces that an item was removed.
    #[must_use]
    pub fn item_removed(label: &str, locale: &Locale, messages: &Messages) -> String {
        (messages.item_removed)(label, locale)
    }

    /// Announces a column sorting change.
    #[must_use]
    pub fn column_sorted(
        column: &str,
        direction: AriaSort,
        locale: &Locale,
        messages: &Messages,
    ) -> String {
        let message = match direction {
            AriaSort::Ascending => &messages.sorted_ascending,
            AriaSort::Descending => &messages.sorted_descending,
            AriaSort::Other => &messages.sorted_other,
            AriaSort::None => &messages.not_sorted,
        };

        message(column, locale)
    }

    /// Announces a tree node expanding or collapsing.
    #[must_use]
    pub fn tree_node_expanded(
        label: &str,
        expanded: bool,
        locale: &Locale,
        messages: &Messages,
    ) -> String {
        let message = if expanded {
            &messages.tree_expanded
        } else {
            &messages.tree_collapsed
        };

        message(label, locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locale() -> Locale {
        Locale::parse("en-US").expect("test locale must parse")
    }

    #[test]
    fn announcement_messages_default_provides_spec_english_templates() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!((messages.search_results)(0, &locale), "No results found.");
        assert_eq!((messages.search_results)(1, &locale), "1 result found.");
        assert_eq!((messages.search_results)(42, &locale), "42 results found.");
        assert_eq!((messages.selected)("Item A", &locale), "Item A, selected.");
        assert_eq!(
            (messages.deselected)("Item A", &locale),
            "Item A, deselected."
        );
        assert_eq!(
            (messages.validation_error)("Email", "is required", &locale),
            "Email: is required. Error."
        );
        assert_eq!((messages.loading)(&locale), "Loading.");
        assert_eq!((messages.loading_complete)(&locale), "Loading complete.");
        assert_eq!(
            (messages.item_moved)("Row 3", 2, 10, &locale),
            "Row 3 moved to position 2 of 10."
        );
        assert_eq!((messages.item_removed)("Tag X", &locale), "Tag X removed.");
        assert_eq!(
            (messages.sorted_ascending)("Name", &locale),
            "Name, sorted ascending."
        );
        assert_eq!(
            (messages.sorted_descending)("Name", &locale),
            "Name, sorted descending."
        );
        assert_eq!((messages.sorted_other)("Name", &locale), "Name, sorted.");
        assert_eq!((messages.not_sorted)("Name", &locale), "Name, not sorted.");
        assert_eq!(
            (messages.tree_expanded)("Folder", &locale),
            "Folder, expanded."
        );
        assert_eq!(
            (messages.tree_collapsed)("Folder", &locale),
            "Folder, collapsed."
        );
    }

    #[test]
    fn search_results_uses_count_aware_template() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::search_results(0, &locale, &messages),
            "No results found."
        );
        assert_eq!(
            Announcements::search_results(1, &locale, &messages),
            "1 result found."
        );
        assert_eq!(
            Announcements::search_results(42, &locale, &messages),
            "42 results found."
        );
    }

    #[test]
    fn selection_changed_uses_selected_and_deselected_templates() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::selection_changed("Item A", true, &locale, &messages),
            "Item A, selected."
        );
        assert_eq!(
            Announcements::selection_changed("Item A", false, &locale, &messages),
            "Item A, deselected."
        );
    }

    #[test]
    fn toast_returns_message_as_is() {
        assert_eq!(Announcements::toast("Hello"), "Hello");
    }

    #[test]
    fn validation_error_replaces_field_and_error_placeholders() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::validation_error("Email", "is required", &locale, &messages),
            "Email: is required. Error."
        );
    }

    #[test]
    fn loading_helpers_invoke_locale_aware_templates() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(Announcements::loading(&locale, &messages), "Loading.");
        assert_eq!(
            Announcements::loading_complete(&locale, &messages),
            "Loading complete."
        );
    }

    #[test]
    fn item_moved_replaces_all_placeholders() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::item_moved("Row 3", 2, 10, &locale, &messages),
            "Row 3 moved to position 2 of 10."
        );
    }

    #[test]
    fn item_removed_replaces_label_placeholder() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::item_removed("Tag X", &locale, &messages),
            "Tag X removed."
        );
    }

    #[test]
    fn column_sorted_handles_all_direction_branches() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::column_sorted("Name", AriaSort::Ascending, &locale, &messages),
            "Name, sorted ascending."
        );
        assert_eq!(
            Announcements::column_sorted("Name", AriaSort::Descending, &locale, &messages),
            "Name, sorted descending."
        );
        assert_eq!(
            Announcements::column_sorted("Name", AriaSort::None, &locale, &messages),
            "Name, not sorted."
        );
        assert_eq!(
            Announcements::column_sorted("Name", AriaSort::Other, &locale, &messages),
            "Name, sorted."
        );
    }

    #[test]
    fn tree_node_expanded_uses_expanded_and_collapsed_templates() {
        let messages = Messages::default();
        let locale = locale();

        assert_eq!(
            Announcements::tree_node_expanded("Folder", true, &locale, &messages),
            "Folder, expanded."
        );
        assert_eq!(
            Announcements::tree_node_expanded("Folder", false, &locale, &messages),
            "Folder, collapsed."
        );
    }

    #[test]
    fn custom_localized_templates_are_used_without_hardcoded_english() {
        let locale = Locale::parse("pt-BR").expect("test locale must parse");
        let messages = Messages {
            search_results: MessageFn::new(Arc::new(|count: usize, locale: &Locale| match count {
                0 => format!("Nenhum resultado ({})", locale.to_bcp47()),
                1 => format!("1 resultado ({})", locale.to_bcp47()),
                _ => format!("{count} resultados ({})", locale.to_bcp47()),
            }) as Arc<CountLocaleMessage>),
            selected: MessageFn::new(Arc::new(|label: &str, locale: &Locale| {
                format!("{label}, selecionado ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            deselected: MessageFn::new(Arc::new(|label: &str, locale: &Locale| {
                format!("{label}, desmarcado ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            validation_error: MessageFn::new(Arc::new(
                |field: &str, error: &str, locale: &Locale| {
                    format!("{field}: {error}. Erro ({})", locale.to_bcp47())
                },
            ) as Arc<FieldErrorLocaleMessage>),
            loading: MessageFn::new(|locale: &Locale| {
                format!("Carregando ({})", locale.to_bcp47())
            }),
            loading_complete: MessageFn::new(|locale: &Locale| {
                format!("Carregamento concluído ({})", locale.to_bcp47())
            }),
            item_moved: MessageFn::new(Arc::new(
                |label: &str, position: usize, total: usize, locale: &Locale| {
                    format!(
                        "{label} movido para a posição {position} de {total} ({})",
                        locale.to_bcp47()
                    )
                },
            ) as Arc<MoveLocaleMessage>),
            item_removed: MessageFn::new(Arc::new(|label: &str, locale: &Locale| {
                format!("{label} removido ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            sorted_ascending: MessageFn::new(Arc::new(|column: &str, locale: &Locale| {
                format!("{column}, ordem crescente ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            sorted_descending: MessageFn::new(Arc::new(|column: &str, locale: &Locale| {
                format!("{column}, ordem decrescente ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            sorted_other: MessageFn::new(Arc::new(|column: &str, locale: &Locale| {
                format!("{column}, ordenado ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            not_sorted: MessageFn::new(Arc::new(|column: &str, locale: &Locale| {
                format!("{column}, sem ordenação ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            tree_expanded: MessageFn::new(Arc::new(|label: &str, locale: &Locale| {
                format!("{label}, expandido ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
            tree_collapsed: MessageFn::new(Arc::new(|label: &str, locale: &Locale| {
                format!("{label}, recolhido ({})", locale.to_bcp47())
            }) as Arc<LabelLocaleMessage>),
        };

        assert_eq!(
            Announcements::search_results(0, &locale, &messages),
            "Nenhum resultado (pt-BR)"
        );
        assert_eq!(
            Announcements::search_results(1, &locale, &messages),
            "1 resultado (pt-BR)"
        );
        assert_eq!(
            Announcements::search_results(3, &locale, &messages),
            "3 resultados (pt-BR)"
        );
        assert_eq!(
            Announcements::selection_changed("Item A", true, &locale, &messages),
            "Item A, selecionado (pt-BR)"
        );
        assert_eq!(
            Announcements::selection_changed("Item A", false, &locale, &messages),
            "Item A, desmarcado (pt-BR)"
        );
        assert_eq!(
            Announcements::validation_error("Email", "obrigatório", &locale, &messages),
            "Email: obrigatório. Erro (pt-BR)"
        );
        assert_eq!(
            Announcements::loading(&locale, &messages),
            "Carregando (pt-BR)"
        );
        assert_eq!(
            Announcements::loading_complete(&locale, &messages),
            "Carregamento concluído (pt-BR)"
        );
        assert_eq!(
            Announcements::item_moved("Linha 3", 2, 10, &locale, &messages),
            "Linha 3 movido para a posição 2 de 10 (pt-BR)"
        );
        assert_eq!(
            Announcements::item_removed("Linha 3", &locale, &messages),
            "Linha 3 removido (pt-BR)"
        );
        assert_eq!(
            Announcements::column_sorted("Nome", AriaSort::Ascending, &locale, &messages),
            "Nome, ordem crescente (pt-BR)"
        );
        assert_eq!(
            Announcements::column_sorted("Nome", AriaSort::Descending, &locale, &messages),
            "Nome, ordem decrescente (pt-BR)"
        );
        assert_eq!(
            Announcements::column_sorted("Nome", AriaSort::Other, &locale, &messages),
            "Nome, ordenado (pt-BR)"
        );
        assert_eq!(
            Announcements::column_sorted("Nome", AriaSort::None, &locale, &messages),
            "Nome, sem ordenação (pt-BR)"
        );
        assert_eq!(
            Announcements::tree_node_expanded("Pasta", true, &locale, &messages),
            "Pasta, expandido (pt-BR)"
        );
        assert_eq!(
            Announcements::tree_node_expanded("Pasta", false, &locale, &messages),
            "Pasta, recolhido (pt-BR)"
        );
    }

    #[test]
    fn search_results_can_vary_by_locale() {
        let en_locale = Locale::parse("en-US").expect("test locale must parse");
        let pt_locale = Locale::parse("pt-BR").expect("test locale must parse");
        let messages = Messages {
            search_results: MessageFn::new(Arc::new(|count: usize, locale: &Locale| {
                match locale.to_bcp47().as_str() {
                    "pt-BR" if count == 0 => String::from("Nenhum resultado encontrado."),
                    "pt-BR" => format!("{count} resultados encontrados."),
                    _ if count == 0 => String::from("No results found."),
                    _ => format!("{count} results found."),
                }
            }) as Arc<CountLocaleMessage>),
            ..Messages::default()
        };

        assert_eq!(
            Announcements::search_results(0, &en_locale, &messages),
            "No results found."
        );
        assert_eq!(
            Announcements::search_results(2, &en_locale, &messages),
            "2 results found."
        );
        assert_eq!(
            Announcements::search_results(0, &pt_locale, &messages),
            "Nenhum resultado encontrado."
        );
        assert_eq!(
            Announcements::search_results(2, &pt_locale, &messages),
            "2 resultados encontrados."
        );
    }

    #[test]
    fn messages_satisfies_component_messages_contract() {
        fn clone_messages<M: ComponentMessages + Clone + Default>(messages: &M) -> M {
            messages.clone()
        }

        let messages = Messages::default();
        let cloned = clone_messages(&messages);
        let locale = locale();

        assert_eq!((cloned.search_results)(0, &locale), "No results found.");
        assert_eq!(Announcements::loading(&locale, &cloned), "Loading.");
    }

    #[test]
    fn cloned_messages_preserve_custom_message_functions() {
        let locale = Locale::parse("pt-BR").expect("test locale must parse");
        let messages = Messages {
            loading: MessageFn::new(|locale: &Locale| format!("Carregando {}", locale.to_bcp47())),
            ..Messages::default()
        };
        let cloned = messages.clone();

        assert_eq!(
            Announcements::loading(&locale, &messages),
            "Carregando pt-BR"
        );
        assert_eq!(Announcements::loading(&locale, &cloned), "Carregando pt-BR");
    }
}
