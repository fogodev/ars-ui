#[cfg(all(feature = "web", target_arch = "wasm32"))]
use std::fmt;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsValue;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub(crate) fn warn_message(args: fmt::Arguments<'_>) {
    #[cfg(feature = "debug")]
    log::warn!("[ars-dom] {args}");

    #[cfg(not(feature = "debug"))]
    let _ = args;
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub(crate) fn warn_skipped(context: &str, missing: &str) {
    warn_message(format_args!(
        "{context} skipped because {missing} is unavailable"
    ));
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub(crate) fn warn_dom_error(context: &str, result: Result<(), JsValue>) {
    #[cfg(feature = "debug")]
    if let Err(error) = &result {
        log::warn!("[ars-dom] {context} failed: {error:?}");
    }

    #[cfg(not(feature = "debug"))]
    let _ = (context, &result);

    drop(result);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub(crate) fn dom_value_or_default<T: Default>(context: &str, result: Result<T, JsValue>) -> T {
    #[cfg(feature = "debug")]
    if let Err(error) = &result {
        log::warn!("[ars-dom] {context} failed: {error:?}");
    }

    #[cfg(not(feature = "debug"))]
    let _ = context;

    result.unwrap_or_default()
}
