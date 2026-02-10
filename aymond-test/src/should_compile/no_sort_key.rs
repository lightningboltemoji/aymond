use aymond::prelude::*;

#[aymond(item, table)]
struct Car {
    #[hash_key]
    make: String,
    hp: i16,
}

fn main() {}
