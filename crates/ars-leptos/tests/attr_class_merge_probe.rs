//! Probe whether Leptos's `attr:class` pass-through merges tokens with a
//! component-supplied class or clobbers it.
//!
//! `VisuallyHidden` already emits `class="ars-visually-hidden"` when
//! `is_focusable=false`. If `attr:class="skip-link"` produces a class
//! attribute containing both tokens, the framework concatenates. If only
//! one token survives, it clobbers.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::utility::visually_hidden::VisuallyHidden;
use leptos::{prelude::*, reactive::owner::Owner};

#[test]
fn probe_attr_class_passthrough_with_existing_component_class() {
    let owner = Owner::new();

    let html = owner.with(|| {
        view! {
            <VisuallyHidden id="probe" attr:class="skip-link">
                "Hidden"
            </VisuallyHidden>
        }
        .to_html()
    });

    println!("==== PROBE OUTPUT ====\n{html}\n==== END ====");
}
