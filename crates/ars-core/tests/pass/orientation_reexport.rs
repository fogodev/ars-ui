use ars_core::Orientation as CoreOrientation;
use ars_i18n::Orientation as I18nOrientation;

fn main() {
    let core_orientation: CoreOrientation = I18nOrientation::Horizontal;
    let i18n_orientation: I18nOrientation = core_orientation;
    let _core_orientation_again: CoreOrientation = i18n_orientation;
}
