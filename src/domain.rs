use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }
}

impl std::ops::Add for Money {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Money {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |left, right| left + right)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCode {
    pub department: String,
    pub fund: String,
    pub subaccount: String,
    pub object_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Draft,
    Submitted,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub status: ApprovalStatus,
    pub requested_by: String,
    pub decided_by: Option<String>,
    pub note: Option<String>,
}

impl Approval {
    pub fn draft(requested_by: impl Into<String>) -> Self {
        Self {
            status: ApprovalStatus::Draft,
            requested_by: requested_by.into(),
            decided_by: None,
            note: None,
        }
    }

    pub fn submit(&mut self) -> Result<(), DomainError> {
        if self.status != ApprovalStatus::Draft {
            return Err(DomainError::InvalidApprovalTransition);
        }
        self.status = ApprovalStatus::Submitted;
        Ok(())
    }

    pub fn approve(&mut self, approver: impl Into<String>) -> Result<(), DomainError> {
        if self.status != ApprovalStatus::Submitted {
            return Err(DomainError::InvalidApprovalTransition);
        }
        self.status = ApprovalStatus::Approved;
        self.decided_by = Some(approver.into());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    pub id: String,
    pub description: String,
    pub amount: Money,
    pub released: Money,
    pub approval: Approval,
}

impl Obligation {
    pub fn open_amount(&self) -> Money {
        self.amount - self.released
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonnelCommitment {
    pub id: String,
    pub role: String,
    pub period: String,
    pub amount: Money,
    pub approval: Approval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actual {
    pub journal_id: String,
    pub description: String,
    pub amount: Money,
    pub posted_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub sequence: u64,
    pub action: String,
    pub actor: String,
    pub previous_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundControl {
    pub id: String,
    pub fiscal_year: u16,
    pub code: AccountCode,
    pub original_budget: Money,
    pub revisions: Vec<Money>,
    pub encumbrances: Vec<Obligation>,
    pub commitments: Vec<Obligation>,
    pub personnel: Vec<PersonnelCommitment>,
    pub actuals: Vec<Actual>,
    pub audit_events: Vec<AuditEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub revised_budget: Money,
    pub encumbered: Money,
    pub committed: Money,
    pub personnel_committed: Money,
    pub actual: Money,
    pub available: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reconciliation {
    pub internal_actual: Money,
    pub statement_actual: Money,
    pub difference: Money,
    pub reconciled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalExportLine {
    pub external_reference: String,
    pub department: String,
    pub fund: String,
    pub subaccount: String,
    pub object_code: String,
    pub amount_cents: i64,
    pub description: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("approval state does not allow this transition")]
    InvalidApprovalTransition,
    #[error("only approved items affect controlled balances")]
    ApprovalRequired,
    #[error("an amount cannot be negative")]
    NegativeAmount,
    #[error("release exceeds the original obligation")]
    ExcessRelease,
}

impl FundControl {
    pub fn new(
        id: impl Into<String>,
        fiscal_year: u16,
        code: AccountCode,
        original_budget: Money,
        actor: &str,
    ) -> Result<Self, DomainError> {
        ensure_nonnegative(original_budget)?;
        let mut control = Self {
            id: id.into(),
            fiscal_year,
            code,
            original_budget,
            revisions: Vec::new(),
            encumbrances: Vec::new(),
            commitments: Vec::new(),
            personnel: Vec::new(),
            actuals: Vec::new(),
            audit_events: Vec::new(),
        };
        control.record("fund_control_created", actor);
        Ok(control)
    }

    pub fn revise_budget(&mut self, amount: Money, actor: &str) {
        self.revisions.push(amount);
        self.record("budget_revised", actor);
    }

    pub fn add_encumbrance(&mut self, item: Obligation, actor: &str) -> Result<(), DomainError> {
        validate_obligation(&item)?;
        self.encumbrances.push(item);
        self.record("encumbrance_added", actor);
        Ok(())
    }

    pub fn add_commitment(&mut self, item: Obligation, actor: &str) -> Result<(), DomainError> {
        validate_obligation(&item)?;
        self.commitments.push(item);
        self.record("commitment_added", actor);
        Ok(())
    }

    pub fn add_personnel(
        &mut self,
        item: PersonnelCommitment,
        actor: &str,
    ) -> Result<(), DomainError> {
        ensure_nonnegative(item.amount)?;
        if item.approval.status != ApprovalStatus::Approved {
            return Err(DomainError::ApprovalRequired);
        }
        self.personnel.push(item);
        self.record("personnel_commitment_added", actor);
        Ok(())
    }

    pub fn post_actual(&mut self, actual: Actual, actor: &str) -> Result<(), DomainError> {
        ensure_nonnegative(actual.amount)?;
        self.actuals.push(actual);
        self.record("actual_posted", actor);
        Ok(())
    }

    pub fn balances(&self) -> BalanceSnapshot {
        let revised_budget = self.original_budget + self.revisions.iter().copied().sum();
        let encumbered = open_approved(&self.encumbrances);
        let committed = open_approved(&self.commitments);
        let personnel_committed = self
            .personnel
            .iter()
            .filter(|item| item.approval.status == ApprovalStatus::Approved)
            .map(|item| item.amount)
            .sum();
        let actual = self.actuals.iter().map(|item| item.amount).sum();
        BalanceSnapshot {
            revised_budget,
            encumbered,
            committed,
            personnel_committed,
            actual,
            available: revised_budget - encumbered - committed - personnel_committed - actual,
        }
    }

    pub fn reconcile(&self, statement_actual: Money) -> Reconciliation {
        let internal_actual = self.balances().actual;
        let difference = statement_actual - internal_actual;
        Reconciliation {
            internal_actual,
            statement_actual,
            difference,
            reconciled: difference == Money::ZERO,
        }
    }

    pub fn export_journals(&self) -> Vec<JournalExportLine> {
        self.actuals
            .iter()
            .map(|actual| JournalExportLine {
                external_reference: actual.journal_id.clone(),
                department: self.code.department.clone(),
                fund: self.code.fund.clone(),
                subaccount: self.code.subaccount.clone(),
                object_code: self.code.object_code.clone(),
                amount_cents: actual.amount.cents(),
                description: actual.description.clone(),
            })
            .collect()
    }

    pub fn verify_audit_chain(&self) -> bool {
        let mut previous = String::new();
        self.audit_events.iter().all(|event| {
            let valid = event.previous_hash == previous
                && event.hash
                    == hash_event(
                        event.sequence,
                        &event.action,
                        &event.actor,
                        &event.previous_hash,
                    );
            previous = event.hash.clone();
            valid
        })
    }

    fn record(&mut self, action: &str, actor: &str) {
        let sequence = self.audit_events.len() as u64 + 1;
        let previous_hash = self
            .audit_events
            .last()
            .map(|event| event.hash.clone())
            .unwrap_or_default();
        let hash = hash_event(sequence, action, actor, &previous_hash);
        self.audit_events.push(AuditEvent {
            sequence,
            action: action.to_owned(),
            actor: actor.to_owned(),
            previous_hash,
            hash,
        });
    }
}

fn validate_obligation(item: &Obligation) -> Result<(), DomainError> {
    ensure_nonnegative(item.amount)?;
    ensure_nonnegative(item.released)?;
    if item.released.cents() > item.amount.cents() {
        return Err(DomainError::ExcessRelease);
    }
    if item.approval.status != ApprovalStatus::Approved {
        return Err(DomainError::ApprovalRequired);
    }
    Ok(())
}

fn ensure_nonnegative(amount: Money) -> Result<(), DomainError> {
    if amount.cents() < 0 {
        Err(DomainError::NegativeAmount)
    } else {
        Ok(())
    }
}

fn open_approved(items: &[Obligation]) -> Money {
    items
        .iter()
        .filter(|item| item.approval.status == ApprovalStatus::Approved)
        .map(Obligation::open_amount)
        .sum()
}

fn hash_event(sequence: u64, action: &str, actor: &str, previous_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.to_be_bytes());
    hasher.update(action.as_bytes());
    hasher.update([0]);
    hasher.update(actor.as_bytes());
    hasher.update([0]);
    hasher.update(previous_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn approved_by(requester: &str, approver: &str) -> Approval {
    let mut approval = Approval::draft(requester);
    approval.submit().expect("draft can be submitted");
    approval
        .approve(approver)
        .expect("submitted request can be approved");
    approval
}

pub fn synthetic_control() -> FundControl {
    let mut control = FundControl::new(
        "FC-DEMO-1",
        2027,
        AccountCode {
            department: "D042".into(),
            fund: "F110".into(),
            subaccount: "S08".into(),
            object_code: "O6100".into(),
        },
        Money::from_cents(50_000_000),
        "budget-admin",
    )
    .expect("synthetic budget is valid");
    control.revise_budget(Money::from_cents(2_500_000), "budget-admin");
    control
        .add_encumbrance(
            Obligation {
                id: "ENC-100".into(),
                description: "Laboratory supplies".into(),
                amount: Money::from_cents(3_000_000),
                released: Money::from_cents(500_000),
                approval: approved_by("coordinator", "director"),
            },
            "coordinator",
        )
        .expect("synthetic encumbrance is valid");
    control
        .add_commitment(
            Obligation {
                id: "COM-200".into(),
                description: "Equipment maintenance".into(),
                amount: Money::from_cents(1_200_000),
                released: Money::ZERO,
                approval: approved_by("coordinator", "director"),
            },
            "coordinator",
        )
        .expect("synthetic commitment is valid");
    control
        .add_personnel(
            PersonnelCommitment {
                id: "PER-300".into(),
                role: "Research assistant".into(),
                period: "2026-09..2027-05".into(),
                amount: Money::from_cents(12_000_000),
                approval: approved_by("manager", "director"),
            },
            "manager",
        )
        .expect("synthetic personnel commitment is valid");
    control
        .post_actual(
            Actual {
                journal_id: "JRN-400".into(),
                description: "Approved supply invoice".into(),
                amount: Money::from_cents(750_000),
                posted_on: "2026-09-15".into(),
            },
            "reconciler",
        )
        .expect("synthetic actual is valid");
    control
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_balance_reserves_every_approved_obligation() {
        let balance = synthetic_control().balances();
        assert_eq!(balance.revised_budget.cents(), 52_500_000);
        assert_eq!(balance.encumbered.cents(), 2_500_000);
        assert_eq!(balance.committed.cents(), 1_200_000);
        assert_eq!(balance.personnel_committed.cents(), 12_000_000);
        assert_eq!(balance.actual.cents(), 750_000);
        assert_eq!(balance.available.cents(), 36_050_000);
    }

    #[test]
    fn invalid_approval_transition_is_rejected() {
        let mut approval = Approval::draft("requester");
        assert_eq!(
            approval.approve("approver"),
            Err(DomainError::InvalidApprovalTransition)
        );
    }

    #[test]
    fn draft_obligation_cannot_affect_balance() {
        let mut control = synthetic_control();
        let result = control.add_commitment(
            Obligation {
                id: "DRAFT".into(),
                description: "Unapproved".into(),
                amount: Money::from_cents(100),
                released: Money::ZERO,
                approval: Approval::draft("requester"),
            },
            "requester",
        );
        assert_eq!(result, Err(DomainError::ApprovalRequired));
    }

    #[test]
    fn reconciliation_reports_exact_integer_difference() {
        let result = synthetic_control().reconcile(Money::from_cents(750_125));
        assert_eq!(result.difference.cents(), 125);
        assert!(!result.reconciled);
    }

    #[test]
    fn audit_chain_is_tamper_evident() {
        let mut control = synthetic_control();
        assert!(control.verify_audit_chain());
        control.audit_events[1].actor = "changed".into();
        assert!(!control.verify_audit_chain());
    }

    #[test]
    fn journals_export_without_becoming_a_general_ledger() {
        let lines = synthetic_control().export_journals();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].external_reference, "JRN-400");
        assert_eq!(lines[0].amount_cents, 750_000);
    }
}
