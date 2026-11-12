//! The logic of the combatlog mod, without game types, so it is tested on its own
//! (`cargo test` here). The `api` crate next to it is the glue to Horizon's hooks.
pub mod abuse;
pub mod admin;
pub mod cmds;
pub mod combat;
pub mod config;
pub mod restrict;
pub mod text;
