//! The sidecar's guts, exposed as a library so other crates can ask what it
//! serves instead of keeping their own copy of the answer.
//!
//! `main.rs` is the stdio loop and nothing else. The one caller that needs
//! this is `surya-harness`: it decides which of these tools a run may call
//! without stopping to ask the user, and that decision has to be checked
//! against the real tool list, not against a list someone typed twice.
//!
//! See `main.rs` for what the server is and how to run it by hand.

pub mod config;
pub mod protocol;

mod browser;
mod cards;
mod mail;
mod shapes;
mod tasks;
