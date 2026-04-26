//! Fieldset inherited field context alias.

/// Framework-context payload propagated from a fieldset to descendant fields.
///
/// This aliases the shared [`crate::field::Context`] type so framework context
/// lookup stays consistent across field-like component trees.
pub type Context = crate::field::Context;
