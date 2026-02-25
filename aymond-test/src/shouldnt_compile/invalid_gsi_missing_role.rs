use aymond::prelude::*;

#[aymond(item, table)]
struct Car {
    #[aymond(hash_key)]
    make: String,
    #[aymond(gsi("by-model"))]
    model: String,
}

fn main() {}
