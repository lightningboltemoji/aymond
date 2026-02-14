use aymond::prelude::*;

#[aymond(item)]
struct Car {
    #[hash_key]
    make: Option<String>,
}

fn main() {}
