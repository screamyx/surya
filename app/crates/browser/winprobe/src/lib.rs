//! Compiles the browser crate's Windows-only modules on the MSVC target
//! from Linux, where the real crate cannot cross-check (psm needs MSVC).
#![allow(dead_code)]
#[path = "../../src/clock.rs"]
mod clock;
