//! A cargo documentation diagnostics-driven editor.
//!
//! whichdoc uses ratatui to give a "diagnostics picker" list of cargo docs `missing_docs` errors,
//! and edtui to emulate a vim editor in which to write your docstrings.
#![allow(clippy::multiple_crate_versions)]

pub mod app_state;
pub mod config;
pub mod edit_plan;
pub mod highlight;
pub mod input;
pub mod types;
pub mod ui;
