use tokio_postgres::{Client, Error, NoTls};

use crate::{FundControl, Money};

pub struct PostgresStore {
    client: Client,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("postgres connection error: {error}");
            }
        });
        Ok(Self { client })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        self.client
            .batch_execute(include_str!("../migrations/0001_initial.sql"))
            .await
    }

    pub async fn save(&mut self, control: &FundControl) -> Result<(), Error> {
        let transaction = self.client.transaction().await?;
        let payload = serde_json::to_value(control).expect("fund control serializes");
        let balance = control.balances();
        transaction
            .execute(
                "INSERT INTO fund_controls
                   (id, fiscal_year, department, fund, subaccount, object_code,
                    revised_budget_cents, available_cents, payload)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                 ON CONFLICT (id) DO UPDATE SET
                   fiscal_year=EXCLUDED.fiscal_year,
                   department=EXCLUDED.department,
                   fund=EXCLUDED.fund,
                   subaccount=EXCLUDED.subaccount,
                   object_code=EXCLUDED.object_code,
                   revised_budget_cents=EXCLUDED.revised_budget_cents,
                   available_cents=EXCLUDED.available_cents,
                   payload=EXCLUDED.payload,
                   updated_at=NOW()",
                &[
                    &control.id,
                    &(control.fiscal_year as i32),
                    &control.code.department,
                    &control.code.fund,
                    &control.code.subaccount,
                    &control.code.object_code,
                    &balance.revised_budget.cents(),
                    &balance.available.cents(),
                    &payload,
                ],
            )
            .await?;
        transaction.commit().await
    }

    pub async fn load(&self, id: &str) -> Result<Option<FundControl>, Error> {
        let row = self
            .client
            .query_opt("SELECT payload FROM fund_controls WHERE id=$1", &[&id])
            .await?;
        Ok(row.map(|row| {
            let payload: serde_json::Value = row.get(0);
            serde_json::from_value(payload).expect("stored payload uses current schema")
        }))
    }

    pub async fn available(&self, id: &str) -> Result<Option<Money>, Error> {
        let row = self
            .client
            .query_opt(
                "SELECT available_cents FROM fund_controls WHERE id=$1",
                &[&id],
            )
            .await?;
        Ok(row.map(|row| Money::from_cents(row.get(0))))
    }
}
