pub mod prelude {
    pub use crate::traits::{Item, NestedItem, Table};
    pub use aymond_derive::{item, nested_item, table};
}

pub mod shim;
pub mod traits;
