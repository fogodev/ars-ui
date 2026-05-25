use ars_core::{ArsContext, ColorMode, StyleStrategy};
use ars_i18n::{Direction, locales};

#[test]
fn ars_provider_core_context_defaults_match_spec() {
    let context = ArsContext::default();

    assert_eq!(context.locale(), &locales::en_us());
    assert_eq!(context.direction(), Direction::Ltr);
    assert_eq!(context.color_mode(), ColorMode::System);
    assert!(!context.disabled());
    assert!(!context.read_only());
    assert_eq!(context.id_prefix(), None);
    assert_eq!(context.portal_container_id(), None);
    assert_eq!(context.root_node_id(), None);
    assert_eq!(*context.style_strategy(), StyleStrategy::Inline);
}
