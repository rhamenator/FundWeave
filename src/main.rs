use fund_weave::{PostgresStore, synthetic_control};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let control = synthetic_control();
    let balance = control.balances();
    println!(
        "control={} revised_budget_cents={} available_cents={} audit_chain_valid={}",
        control.id,
        balance.revised_budget.cents(),
        balance.available.cents(),
        control.verify_audit_chain()
    );
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let mut store = PostgresStore::connect(&database_url).await?;
        store.migrate().await?;
        store.save(&control).await?;
        println!("persisted_to_postgresql=true");
    } else {
        println!("persisted_to_postgresql=false (set DATABASE_URL to enable)");
    }
    Ok(())
}
