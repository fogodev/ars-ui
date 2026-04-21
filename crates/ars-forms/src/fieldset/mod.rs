//! Fieldset component machine and inherited field context alias.

pub mod component;

/// Framework-context payload propagated from a fieldset to descendant fields.
///
/// This aliases the shared [`crate::field::Context`] type so framework context
/// lookup stays consistent across `Fieldset`, `CheckboxGroup`, `RadioGroup`,
/// and descendant `Field` components.
pub type Context = crate::field::Context;
