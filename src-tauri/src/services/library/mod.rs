mod contracts;
mod health;
mod identity;
mod importer;
mod launch;
mod provenance;
mod repository;

pub use contracts::*;
pub use health::library_health;
pub use identity::{
    find_identity_matches, normalize_launch_path, stable_candidate_id, title_fingerprint,
};
pub use importer::{apply_import, build_candidate, preview_import, LibraryImportBackend};
pub use launch::{launch, launch_descriptor};
pub use provenance::{plan_field_diffs, ProvenanceLedger};
pub use repository::DatabaseLibraryBackend;

#[cfg(test)]
mod tests;
