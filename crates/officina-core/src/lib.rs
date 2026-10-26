//! Core domain logic for Assetto Corsa: paths, content scanning, INI, race.ini,
//! launching, mod installation. No UI dependencies.

pub const AC_APP_ID: u32 = 244210;

pub mod acd;
pub mod backup;
pub mod content;
pub mod csp_config;
pub mod events;
pub mod game_cfg;
pub mod ini;
pub mod install;
pub mod json;
pub mod launch;
pub mod mods;
pub mod paths;
pub mod race;
pub mod settings;
pub mod setup;
pub mod setups;
pub mod trash;
pub mod validate;
