//! External sources behind one boundary: the contracts grove reads as data.
//!
//! A `Source` runs one command and turns its JSON into facts keyed by checkout path. Nothing outside this module
//! knows the contracts' shapes. Outcomes:
//!
//! - `absent`: the tool is not on `PATH`. Its facts are `unknown` and this alone is not partial (exit 0).
//! - `failed`: the tool ran but failed, or produced JSON that does not match the contract. Its facts are
//!   `unknown` and the exit code is 3.
//! - `ok`.
//!
//! Status (2026-11-21): waiting on the JSON Schemas and golden fixtures from graft's repository.

mod graft;
mod pot;
