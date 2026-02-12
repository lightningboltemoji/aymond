use aymond::prelude::*;

mod integ;

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

#[tokio::main]
async fn main() {
    let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
        variants: vec![
            "Carrera".into(),
            "Carrera S".into(),
            "Carrera 4S".into(),
            "GT3 RS".into(),
        ],
        production: Production {
            began: 1964,
            units: 1_100_000,
        },
    };
    table.put().item(it).send().await.expect("Failed to write");

    let _ = table.get().make("Porsche").model("911").send().await;
}
