//! `ah doctor`: genesis doctor framework plus espectacular-specific checks,
//! framework detection, capability suggestions, and advisory session steps.

mod build;
mod checks;
mod corpus;
mod detect;
mod enable;
mod envelope;
mod hooks;
mod runner;
mod session;
mod types;

#[cfg(test)]
mod lint_session_tests;
#[cfg(test)]
mod tests;

pub use enable::run_doctor_enable;
pub use envelope::doctor_to_envelope;
pub use runner::run_doctor;
pub use session::record_lint_run;
pub use types::{DoctorEnableResult, DoctorOutcome};

#[cfg(test)]
pub(crate) use session::lint_marker_path;
