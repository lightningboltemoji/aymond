use aymond::prelude::*;

#[aymond(item)]
struct Car {
    #[hash_key]
    make: String,
    #[sort_key]
    model: Option<String>,
}

fn main() {}
