//! Leptos root-reassignment helper for the `as_child` pattern.
//!
//! The slot receives converted Leptos attributes from a hosting component and
//! applies them to one typed child root. It deliberately uses [`TypedChildren`]
//! plus [`AddAnyAttr`] instead of opaque child vnode mutation, which is not a
//! stable Leptos composition surface.

use leptos::{children::TypedChildren, prelude::*, tachys::view::add_attr::AddAnyAttr};

use crate::LeptosAttribute;

/// Applies converted Leptos attributes to a typed child without rendering an
/// adapter wrapper node.
///
/// Hosting components are responsible for building the final merged
/// framework-agnostic attrs and converting them according to the active style
/// strategy before calling this slot. The slot renders the typed child root and
/// attaches the converted attrs through [`AddAnyAttr`].
#[component]
pub fn AsChildSlot<T>(
    /// Converted Leptos attributes produced by the hosting component.
    attrs: Vec<LeptosAttribute>,
    /// Typed child root that receives the converted attributes.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: AddAnyAttr,
    <View<T> as AddAnyAttr>::Output<Vec<LeptosAttribute>>: IntoView,
{
    let child = children.into_inner()();

    child.add_any_attr(attrs)
}
