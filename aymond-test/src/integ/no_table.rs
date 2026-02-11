#[tokio::test]
async fn test() {
    use aymond::prelude::*;
    use std::collections::HashMap;

    #[aymond(item)]
    struct Car {
        #[hash_key]
        make: String,
        hp: i16,
    }

    let it = Car {
        make: "Porsche".to_string(),
        hp: 518,
    };
    let _: HashMap<_, _> = it.into();
}
