use aymond::prelude::*;

#[aymond(item)]
struct Car {
    #[aymond(hash_key)]
    make: String,
    #[aymond(sort_key)]
    model: Option<String>,
}

fn main() {}
