//! Dioxus `VisuallyHidden` adapter.
//!
//! Renders the framework-agnostic `VisuallyHidden` attribute contract as either
//! an adapter-owned `<span>` root or a consumer-owned root through
//! [`VisuallyHiddenAsChild`].

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

pub use ars_components::utility::visually_hidden::{Api, Part, Props};
use ars_core::HtmlAttr;
use dioxus::{
    dioxus_core::{AttributeValue, Template, TemplateAttribute, TemplateNode, VNode},
    prelude::*,
};

use crate::{
    as_child::{AsChildRenderProps, merge_dioxus_attrs},
    attr_map_to_dioxus_inline_attrs,
};

static TEMPLATE_CACHE: LazyLock<Mutex<HashMap<TemplateKey, Template>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TemplateKey {
    roots: usize,
    node_paths: usize,
    attr_paths: usize,
}

fn root_attrs(id: Option<&str>, is_focusable: bool, as_child: bool) -> Vec<Attribute> {
    let mut props = Props::new().is_focusable(is_focusable).as_child(as_child);

    if let Some(id) = id {
        props = props.id(id);
    }

    let mut attrs = Api::new(props).root_attrs();

    if let Some(id) = id {
        attrs.set(HtmlAttr::Id, id);
    }

    attr_map_to_dioxus_inline_attrs(attrs)
}

/// Props for the Dioxus [`VisuallyHidden`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct VisuallyHiddenProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the hidden content should become visible when focused.
    #[props(default = false)]
    pub is_focusable: bool,

    /// Hidden content that remains available to assistive technology.
    pub children: Element,
}

/// Props for the Dioxus [`VisuallyHiddenAsChild`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct VisuallyHiddenAsChildProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the hidden content should become visible when focused.
    #[props(default = false)]
    pub is_focusable: bool,

    /// Render callback that owns the child root and spreads `VisuallyHidden` attrs.
    pub render: Callback<AsChildRenderProps, Element>,
}

/// Dioxus `VisuallyHidden` component rendered as an adapter-owned `<span>` root.
#[component]
pub fn VisuallyHidden(props: VisuallyHiddenProps) -> Element {
    let attrs = root_attrs(props.id.as_deref(), props.is_focusable, false);
    let children = props.children;

    rsx! {
        span { ..attrs,{children} }
    }
}

/// Dioxus `VisuallyHidden` component that forwards root attrs to one child root.
#[component]
pub fn VisuallyHiddenAsChild(props: VisuallyHiddenAsChildProps) -> Element {
    let mut attrs = root_attrs(props.id.as_deref(), props.is_focusable, true);
    let hidden_class = (!props.is_focusable)
        .then(|| take_attr(&mut attrs, "class"))
        .flatten();

    props
        .render
        .call(AsChildRenderProps { attrs })
        .map(|vnode| merge_root_attrs(vnode, hidden_class))
}

fn take_attr(attrs: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    let index = attrs.iter().position(|attr| attr.name == name)?;

    Some(attrs.remove(index))
}

fn merge_root_attrs(vnode: VNode, attr: Option<Attribute>) -> VNode {
    let Some(attr) = attr else {
        return vnode;
    };

    let (template, merged_static_class) = merge_static_root_class(vnode.template, &attr);

    let dynamic_attrs = if merged_static_class {
        vnode.dynamic_attrs.clone()
    } else if let Some((first, rest)) = vnode.dynamic_attrs.split_first() {
        let first = first.to_vec();
        let first = merge_dioxus_attrs(first, vec![attr]).into_boxed_slice();

        std::iter::once(first)
            .chain(rest.iter().cloned())
            .collect::<Vec<_>>()
            .into_boxed_slice()
    } else {
        vnode.dynamic_attrs.clone()
    };

    VNode::new(
        vnode.key.clone(),
        template,
        vnode.dynamic_nodes.clone(),
        dynamic_attrs,
    )
}

fn merge_static_root_class(template: Template, attr: &Attribute) -> (Template, bool) {
    if attr.name != "class" {
        return (template, false);
    }

    let key = TemplateKey::from(template);

    if let Some(template) = TEMPLATE_CACHE
        .lock()
        .expect("template cache lock should not be poisoned")
        .get(&key)
        .copied()
    {
        return (template, true);
    }

    let Some((root, rest)) = template.roots.split_first() else {
        return (template, false);
    };

    let (root, merged) = merge_static_class_into_node(*root, attr);

    if !merged {
        return (template, false);
    }

    let roots = std::iter::once(root)
        .chain(rest.iter().copied())
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let template = Template {
        roots: Box::leak(roots),
        node_paths: template.node_paths,
        attr_paths: template.attr_paths,
    };

    TEMPLATE_CACHE
        .lock()
        .expect("template cache lock should not be poisoned")
        .insert(key, template);

    (template, true)
}

fn merge_static_class_into_node(node: TemplateNode, attr: &Attribute) -> (TemplateNode, bool) {
    let TemplateNode::Element {
        tag,
        namespace,
        attrs,
        children,
    } = node
    else {
        return (node, false);
    };

    let mut merged = false;

    let attrs = attrs
        .iter()
        .map(|existing| match existing {
            TemplateAttribute::Static {
                name: "class",
                value,
                namespace: None,
            } => {
                merged = true;

                TemplateAttribute::Static {
                    name: "class",
                    value: leaked_class_tokens(value, attr),
                    namespace: None,
                }
            }

            existing => clone_template_attr(existing),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    (
        TemplateNode::Element {
            tag,
            namespace,
            attrs: Box::leak(attrs),
            children,
        },
        merged,
    )
}

const fn clone_template_attr(attr: &TemplateAttribute) -> TemplateAttribute {
    match attr {
        TemplateAttribute::Static {
            name,
            value,
            namespace,
        } => TemplateAttribute::Static {
            name,
            value,
            namespace: *namespace,
        },

        TemplateAttribute::Dynamic { id } => TemplateAttribute::Dynamic { id: *id },
    }
}

fn leaked_class_tokens(existing: &'static str, attr: &Attribute) -> &'static str {
    let AttributeValue::Text(component) = &attr.value else {
        return existing;
    };

    let mut tokens = existing.split_whitespace().collect::<Vec<_>>();

    for token in component.split_whitespace() {
        if !tokens.contains(&token) {
            tokens.push(token);
        }
    }

    Box::leak(tokens.join(" ").into_boxed_str())
}

impl From<Template> for TemplateKey {
    fn from(template: Template) -> Self {
        Self {
            roots: template.roots.as_ptr() as usize,
            node_paths: template.node_paths.as_ptr() as usize,
            attr_paths: template.attr_paths.as_ptr() as usize,
        }
    }
}
