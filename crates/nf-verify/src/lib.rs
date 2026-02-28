// nf-verify: Auto-verification against source APIs

pub mod crossref;
pub mod error;
pub mod fec;
pub mod pipeline;
pub mod result;
pub mod url;

pub use error::{VerifyError, VerifyResult};
pub use pipeline::{
    BatchStats, VerifyConfig, VerifyOutput, compute_batch_stats, verify_batch,
    verify_person_by_name, verify_source, verify_source_with_result,
};
pub use result::{ConfidenceLevel, Evidence, VerificationResult};
pub use url::{
    ArchiveSaveResult, FreshnessResult, archive_url_wayback, archive_url_wayback_with_base,
    check_freshness,
};
