use fund_weave::{PostgresStore, synthetic_control};

#[tokio::test]
async fn roundtrips_control_and_available_balance() {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let mut store = PostgresStore::connect(&database_url).await.unwrap();
    store.migrate().await.unwrap();
    let control = synthetic_control();
    store.save(&control).await.unwrap();
    let loaded = store.load(&control.id).await.unwrap().unwrap();
    assert_eq!(loaded, control);
    assert_eq!(
        store.available(&control.id).await.unwrap().unwrap().cents(),
        36_050_000
    );
}
