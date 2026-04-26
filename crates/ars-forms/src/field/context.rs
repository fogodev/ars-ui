//! Shared context for child fields within a parent group.
//!
//! [`Context`] is the bridge between a parent group component (`Fieldset`,
//! `CheckboxGroup`, `RadioGroup`) and child `Field` components. The parent
//! provides it via framework context; child `Field` components consume it to
//! inherit disabled/invalid/readonly state.

use alloc::string::String;

/// Propagated via framework context (Leptos `provide_context`, Dioxus
/// `use_context_provider`) from `Fieldset` (or `CheckboxGroup`, `RadioGroup`)
/// to child Field components.
///
/// **Merge semantics**: When a `Field` detects a parent `Context`, it
/// merges via logical OR:
///
/// - `effective_disabled = field_props.disabled || field_ctx.disabled`
/// - `effective_invalid = field_props.invalid || field_ctx.invalid`
/// - `effective_readonly = field_props.readonly || field_ctx.readonly`
///
/// The merge happens at the adapter layer (not inside the core machine),
/// because `ars_components::utility::field::Machine` has no access to
/// framework context.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// Optional field name inherited from the parent group (e.g., checkbox group name).
    pub name: Option<String>,

    /// Whether the parent group is disabled. Child fields merge this with
    /// their own `disabled` prop.
    pub disabled: bool,

    /// Whether the parent group is invalid. Child fields merge this with
    /// their own `invalid` prop.
    pub invalid: bool,

    /// Whether the parent group is read-only. Child fields merge this with
    /// their own `readonly` prop.
    pub readonly: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_false() {
        let ctx = Context::default();

        assert!(ctx.name.is_none());
        assert!(!ctx.disabled);
        assert!(!ctx.invalid);
        assert!(!ctx.readonly);
    }

    #[test]
    fn merge_semantics_logical_or() {
        // Simulate the merge that adapters perform
        let parent_ctx = Context {
            name: Some("group".into()),
            disabled: true,
            invalid: false,
            readonly: true,
        };

        let field_disabled = false;
        let field_invalid = true;
        let field_readonly = false;

        let effective_disabled = field_disabled || parent_ctx.disabled;
        let effective_invalid = field_invalid || parent_ctx.invalid;
        let effective_readonly = field_readonly || parent_ctx.readonly;

        assert!(effective_disabled); // parent is disabled
        assert!(effective_invalid); // field is invalid
        assert!(effective_readonly); // parent is readonly
    }
}
