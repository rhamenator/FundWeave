# FundWeave

FundWeave is a clean-room departmental fund-control core. It reserves approved
encumbrances, operating commitments, and personnel commitments against revised
budgets; records actuals; calculates exact available balances in integer cents;
reconciles statement totals; and exports neutral journal lines for a separate
general ledger.

```powershell
cargo run
docker compose up -d
$env:DATABASE_URL = "postgresql://fundweave:fundweave@localhost:54330/fundweave"
cargo test --all-targets
```

The demonstration and tests use fictional codes, actors, amounts, and dates.
FundWeave is deliberately not a general ledger: it controls departmental funds
and emits journal lines through an integration boundary.

See [Architecture](docs/ARCHITECTURE.md) and
[Clean-room boundary](docs/CLEAN_ROOM_BOUNDARY.md).
