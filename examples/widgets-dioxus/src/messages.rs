use std::sync::Arc;

use ars_dioxus::{
    I18nRegistries, MessageFn, MessagesRegistry,
    navigation::tabs,
    utility::{button, dismissable, error_boundary},
};

/// Builds the example-local component message registries.
pub(crate) fn i18n_registries() -> Arc<I18nRegistries> {
    let mut registries = I18nRegistries::new();

    registries.register(MessagesRegistry::new(button::Messages::default()).register(
        "pt-BR",
        button::Messages {
            loading_label: MessageFn::static_str("Carregando"),
        },
    ));

    registries.register(MessagesRegistry::new(tabs::Messages::default()).register(
        "pt-BR",
        tabs::Messages {
            close_tab_label: MessageFn::new(|label: &str, _locale: &ars_dioxus::Locale| {
                format!("Fechar {label}")
            }),
            reorder_announce_label: MessageFn::new(
                |label: &str, position: usize, total: usize, _locale: &ars_dioxus::Locale| {
                    format!("Aba {label} movida para a posição {position} de {total}")
                },
            ),
        },
    ));

    registries.register(
        MessagesRegistry::new(dismissable::Messages::default()).register(
            "pt-BR",
            dismissable::Messages {
                dismiss_label: MessageFn::static_str("Dispensar"),
            },
        ),
    );

    registries.register(
        MessagesRegistry::new(error_boundary::Messages::default()).register(
            "pt-BR",
            error_boundary::Messages {
                message: MessageFn::static_str("Um componente encontrou um erro."),
            },
        ),
    );

    Arc::new(registries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_pt_br_component_messages() {
        let registries = i18n_registries();

        let locale = ars_dioxus::Locale::parse("pt-BR").expect("pt-BR locale");

        let button = registries
            .get::<button::Messages>()
            .expect("button messages");

        assert_eq!(
            button.get(&locale).loading_label.as_ref()(&locale),
            "Carregando"
        );

        let tabs = registries.get::<tabs::Messages>().expect("tabs messages");

        assert_eq!(
            tabs.get(&locale).close_tab_label.as_ref()("Teclado", &locale),
            "Fechar Teclado"
        );
        assert_eq!(
            tabs.get(&locale).reorder_announce_label.as_ref()("Teclado", 2, 4, &locale),
            "Aba Teclado movida para a posição 2 de 4"
        );

        let dismissable = registries
            .get::<dismissable::Messages>()
            .expect("dismissable messages");

        assert_eq!(
            dismissable.get(&locale).dismiss_label.as_ref()(&locale),
            "Dispensar"
        );

        let error_boundary = registries
            .get::<error_boundary::Messages>()
            .expect("error boundary messages");

        assert_eq!(
            error_boundary.get(&locale).message.as_ref()(&locale),
            "Um componente encontrou um erro."
        );
    }
}
