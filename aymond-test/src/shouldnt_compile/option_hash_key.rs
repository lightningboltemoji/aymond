use aymond::prelude::*;

#[aymond(item)]
struct Car {
    #[aymond(hash_key)]
    make: Option<String>,
}

fn main() {}
