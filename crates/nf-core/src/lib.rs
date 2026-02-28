// nf-core: FollowTheMoney data model in Rust
// Every entity requires a SourceRef chain — enforced at the type level

pub mod entities;
pub mod error;
pub mod relationships;
pub mod schema;
pub mod source;

pub use entities::*;
pub use error::*;
pub use relationships::*;
pub use source::*;
