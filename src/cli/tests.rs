//! Tests for the pure CLI helper logic, split by concern to keep files small.
//!
//! Each chunk module brings the code under test into scope with
//! `use crate::cli::*;` and any shared helpers with `use super::*;`.

mod mode_and_output;
mod parsing_and_format;
