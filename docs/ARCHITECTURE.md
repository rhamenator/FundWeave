# Architecture

The domain layer owns integer-cent budgets, approvals, obligations, available
balance calculation, reconciliation, journal export, and a SHA-256-linked audit
trail. PostgreSQL is an adapter: it stores a queryable account key and current
snapshot while the domain remains independently testable.

The initial vertical slice uses a transactional snapshot. A production service
can later add authenticated commands and append-only event storage without
changing the calculation boundary. General-ledger posting remains external;
FundWeave exports neutral journal lines for an accounting integration to accept
or reject.
