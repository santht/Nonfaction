// nf-verify: Auto-verification against source APIs

pub mod error;
pub mod fec;
pub mod url;
pub mod crossref;
pub mod pipeline;

pub use error::{VerifyError, VerifyResult};
pub use pipeline::{VerifyConfig, VerifyOutput, verify_person_by_name, verify_source};
