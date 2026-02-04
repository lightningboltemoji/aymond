pub mod prelude {
    pub use crate::traits::{Item, Table};
    pub use dynamodb_enhanced_derive::{item, table};
}

pub mod shim;
pub mod traits;
