pub mod prelude {
    pub use crate::traits::{Item, NestedItem, Table};
    pub use aymond_derive::aymond;
}

pub mod shim;
pub mod traits;
