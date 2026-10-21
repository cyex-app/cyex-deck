//! Mod tools logic without I/O (spec 09): mod commands (`command`), durations,
//! `/nuke` matching (M1, `nuke`), pacing for mass actions (`pacer`) and mod-action lines
//! for the "Mod actions" panels (V103-15, `actions`); who may do what (V107-R14, `may`);
//! mass ban outcomes (V107-R21, `mass`); every channel role change (V107-R42, `roles`).

pub mod actions;
pub mod command;
pub mod duration;
pub mod log;
pub mod mass;
pub mod mass_jobs;
pub mod mass_run;
pub mod may;
pub mod nuke;
pub mod own;
pub mod pacer;
pub mod restack;
pub mod roles;
