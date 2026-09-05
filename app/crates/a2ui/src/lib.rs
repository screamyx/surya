//! surya-a2ui — agent-drawn cards (surya decision 4), rendered natively.
//!
//! A2UI v0.9.1 (github.com/a2ui-project/a2ui, Apache-2.0) is a JSON spec: an
//! agent sends a flat component list from a fixed catalog plus a data model,
//! the client rebuilds the tree by id and renders it with its own components,
//! and user actions go back as events. This crate is surya's client side:
//!
//! - [`model`] — the typed component tree (`Card`, `Component`, `Dynamic`).
//! - [`parse`] — the message shape (`createSurface` / `updateComponents` /
//!   `updateDataModel` envelopes, or the `{components, data}` shorthand a
//!   `show_card` tool call carries) into a [`model::Card`]. Never panics:
//!   malformed input becomes a card whose only content is its diagnostics.
//! - [`data`] — the data model with JSON Pointer paths, relative template
//!   scopes, and the few catalog functions a card needs (`formatString`…).
//! - [`state`] — per-card local state (two-way bindings, tab selection,
//!   field focus) and the [`state::CardEvent`]s a render emits.
//! - [`theme`] — the token set the renderer paints with; the host fills it
//!   from its own theme so a card never carries colors of its own.
//! - [`images`] — where an Image may load from (data: and workspace files;
//!   remote stays a placeholder unless the host allows it).
//! - [`budget`] — the per-card element cap that bounds template fan-out.
//! - [`render`] / [`leaf`] — the GPUI renderer: Text, Image, Button,
//!   TextField, CheckBox, Row, Column, List, Card, Divider, Tabs, plus a
//!   labelled fallback box for anything else.
//!
//! The wire form of a button press (`[card:<id>] <action> <payload>`) lives
//! in `zeron-proto` ([`zeron_proto::CardAction`]) so the engine and harness
//! share it without depending on this crate.

pub mod budget;
pub mod data;
pub mod images;
pub mod inline;
pub mod leaf;
pub mod model;
pub mod parse;
pub mod render;
pub mod state;
pub mod theme;

pub use model::{Card, Component, ComponentKind};
pub use parse::{parse_card, parse_card_str};
pub use budget::{Budget, MAX_NODES};
pub use images::{ImageDecision, ImagePolicy};
pub use render::Renderer;
pub use state::{Binding, CardEvent, CardState};
pub use theme::CardTheme;
pub use zeron_proto::CardAction;
