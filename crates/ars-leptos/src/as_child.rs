//! Leptos root-reassignment helper for the `as_child` pattern.
//!
//! The slot receives converted Leptos attributes from a hosting component and
//! applies them to one typed child root. It deliberately uses [`TypedChildren`]
//! plus [`AddAnyAttr`] instead of opaque child vnode mutation, which is not a
//! stable Leptos composition surface.

use ars_components::utility::as_child::AsChildMerge;
use ars_core::AttrMap;
use leptos::{children::TypedChildren, prelude::*, tachys::view::add_attr::AddAnyAttr};

use crate::{LeptosAttribute, attr_map_to_leptos_inline_attrs};

/// Converted attributes ready for a Leptos `as_child` slot.
///
/// Use [`Self::from_merged_attr_maps`] when a hosting component has both
/// framework-agnostic component attrs and framework-agnostic child attrs. That
/// keeps `class`, `style`, and tokenized ARIA merging in [`AsChildMerge`]
/// before the attrs become opaque Leptos values.
#[derive(Clone, Debug)]
pub struct AsChildAttrs {
    attrs: Vec<LeptosAttribute>,
}

impl AsChildAttrs {
    /// Converts an already merged [`AttrMap`] into inline Leptos attributes.
    #[must_use]
    pub fn from_attr_map(attrs: AttrMap) -> Self {
        Self {
            attrs: attr_map_to_leptos_inline_attrs(attrs),
        }
    }

    /// Merges component attrs onto child attrs, then converts the result.
    ///
    /// This is the supported path when the child root contributes mergeable
    /// attrs such as `class`, `style`, `aria-labelledby`, or
    /// `aria-describedby`. Literal attrs already baked into the typed Leptos
    /// child cannot be inspected after rendering, so component adapters should
    /// collect child attrs as an [`AttrMap`] before calling the slot.
    #[must_use]
    pub fn from_merged_attr_maps(component_attrs: AttrMap, child_attrs: AttrMap) -> Self {
        Self::from_attr_map(component_attrs.merge_onto(child_attrs))
    }

    /// Returns the converted Leptos attributes.
    #[must_use]
    pub fn into_inner(self) -> Vec<LeptosAttribute> {
        self.attrs
    }
}

impl From<Vec<LeptosAttribute>> for AsChildAttrs {
    fn from(attrs: Vec<LeptosAttribute>) -> Self {
        Self { attrs }
    }
}

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
    #[prop(into)]
    attrs: AsChildAttrs,
    /// Typed child root that receives the converted attributes.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: AddAnyAttr,
    <View<T> as AddAnyAttr>::Output<Vec<LeptosAttribute>>: IntoView,
{
    let child = children.into_inner()();

    child.add_any_attr(attrs.into_inner())
}
