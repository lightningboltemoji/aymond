use aymond::prelude::*;

#[aymond(item, table)]
struct Car {
    #[hash_key]
    make: String,
    #[sort_key]
    model: String,
    hp: i16,
    variants: Vec<String>,
    production: Production,
}

#[aymond(nested_item)]
struct Production {
    began: i32,
    #[attribute(name = "units_produced")]
    units: i64,
}

fn main() {}
