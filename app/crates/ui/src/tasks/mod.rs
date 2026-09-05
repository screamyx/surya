//! Task board pane (decision 11 feature 3). Self-contained: the shell mounts
//! [`TasksPane::new`] with the engine client and a space id; nothing here
//! touches `shell.rs`. `demo` is the `zeron --tasks-demo` entry.

pub mod board;
mod card;
mod chips;
pub mod demo;
mod edit;
pub mod model;

pub use board::TasksPane;
