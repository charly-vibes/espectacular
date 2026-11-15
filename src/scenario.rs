//! Scenario authoring commands: `ah scenario new` and `ah scenario supersede`.
//!
//! The module root only wires the parts together; the implementation lives in
//! `scenario/new.rs` (append a scenario skeleton and stage its contract) and
//! `scenario/supersede.rs` (overlay a superseded deployed contract).

mod new;
mod supersede;

pub use new::run_scenario_new;
pub use supersede::run_scenario_supersede;
