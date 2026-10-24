//! tool_core: every rule of the game.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.

pub mod battle;
pub mod content;
pub mod encounter;
pub mod fx;
pub mod geom;
pub mod glossary;
pub mod look;
pub mod mana;
pub mod mesh;
pub mod player;
pub mod quest;
pub mod render;
pub mod rng;
pub mod rules;
pub mod scene;
pub mod session;
pub mod story;
pub mod view;
pub mod web;
pub mod world;
