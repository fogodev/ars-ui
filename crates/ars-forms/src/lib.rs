//! Form validation, field state management, and form context.
//!
//! This crate provides the validation primitives, field tracking types, form
//! context management, field association helpers, and hidden input utilities
//! used by form-related components (text fields, checkboxes, selects, etc.).
//!
//! # Modules
//!
//! - **[`field`]** — [`field::State`], [`field::Value`], [`field::Context`],
//!   [`field::Descriptors`], [`field::InputAria`], [`field::ValueExt`],
//!   [`field::component::Machine`]
//! - **[`fieldset`]** — [`fieldset::Context`],
//!   [`fieldset::component::Machine`], [`fieldset::component::Props`],
//!   [`fieldset::component::Part`]
//! - **[`validation`]** — [`validation::Error`], [`validation::Result`],
//!   [`validation::ResultExt`], [`validation::Validator`],
//!   [`validation::BoxedValidator`], [`validation::Context`],
//!   [`validation::AsyncValidator`]
//! - **[`form`]** — [`form::Context`], [`form::Data`], [`form::Mode`],
//!   [`form::CrossFieldValidator`], [`form::AnyValidator`],
//!   [`form::Messages`], [`form::component::Machine`]
//! - **[`hidden_input`]** — [`hidden_input::Config`], [`hidden_input::Value`],
//!   [`hidden_input::attrs()`], [`hidden_input::multi_attrs()`]

pub mod field;
pub mod fieldset;
pub mod form;
pub mod form_submit;
pub mod hidden_input;
pub mod validation;
