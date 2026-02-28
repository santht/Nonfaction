// nf-core: FollowTheMoney data model in Rust
// Every entity requires a SourceRef chain — enforced at the type level

pub mod entities;
pub mod relationships;
pub mod source;
pub mod schema;
pub mod error;

pub use entities::*;
pub use relationships::*;
pub use source::*;
pub use error::*;
