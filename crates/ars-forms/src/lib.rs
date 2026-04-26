//! Form validation, field state management, and form context.
//!
//! This crate provides the validation primitives, field tracking types, form
//! context management, field association helpers, and hidden input utilities
//! used by form-related components.
//!
//! # Modules
//!
//! - **[`field`]** — [`field::State`], [`field::Value`], [`field::Context`],
//!   [`field::Descriptors`], [`field::InputAria`], [`field::ValueExt`]
//! - **[`fieldset`]** — [`fieldset::Context`]
//! - **[`validation`]** — [`validation::Error`], [`validation::Result`],
//!   [`validation::ResultExt`], [`validation::Validator`],
//!   [`validation::BoxedValidator`], [`validation::Context`],
//!   [`validation::AsyncValidator`]
//! - **[`form`]** — [`form::Context`], [`form::Data`], [`form::Mode`],
//!   [`form::CrossFieldValidator`], [`form::AnyValidator`],
//!   [`form::Messages`]
//! - **[`hidden_input`]** — [`hidden_input::Config`], [`hidden_input::Value`],
//!   [`hidden_input::attrs()`], [`hidden_input::multi_attrs()`]
//!
//! Framework-agnostic component machines now live in `ars-components`.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod field;
pub mod fieldset;
pub mod form;
pub mod hidden_input;
pub mod validation;
