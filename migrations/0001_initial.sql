CREATE TABLE IF NOT EXISTS fund_controls (
    id TEXT PRIMARY KEY,
    fiscal_year INTEGER NOT NULL,
    department TEXT NOT NULL,
    fund TEXT NOT NULL,
    subaccount TEXT NOT NULL,
    object_code TEXT NOT NULL,
    revised_budget_cents BIGINT NOT NULL,
    available_cents BIGINT NOT NULL,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS fund_controls_lookup
    ON fund_controls (fiscal_year, department, fund, subaccount, object_code);
