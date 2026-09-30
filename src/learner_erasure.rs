use crate::infrai_control::{InfraiClient, InfraiError};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourseDelivery {
    pub learner_id: String,
    pub course_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnerDeadline {
    pub learner_id: String,
    pub course_id: String,
    pub due_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EducatorReportRow {
    pub learner_id: String,
    pub course_id: String,
    pub completion_percent: u8,
}

#[derive(Default)]
pub struct LearningRecords {
    pub deliveries: Vec<CourseDelivery>,
    pub deadlines: Vec<LearnerDeadline>,
    pub report_rows: Vec<EducatorReportRow>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ErasureReceipt {
    pub user_id: String,
    pub sessions_revoked: usize,
    pub deliveries_removed: usize,
    pub deadlines_removed: usize,
    pub report_rows_removed: usize,
    pub credential_revoked: bool,
}

#[derive(Debug, Error)]
pub enum ErasureError {
    #[error("control credential and target credential must differ")]
    UnsafeCredentialTarget,
    #[error("Infrai request failed: {0}")]
    Infrai(#[from] InfraiError),
}

impl LearningRecords {
    pub fn erase_learner(&mut self, user_id: &str, sessions_revoked: usize) -> ErasureReceipt {
        let before_deliveries = self.deliveries.len();
        let before_deadlines = self.deadlines.len();
        let before_reports = self.report_rows.len();

        self.deliveries.retain(|row| row.learner_id != user_id);
        self.deadlines.retain(|row| row.learner_id != user_id);
        self.report_rows.retain(|row| row.learner_id != user_id);

        ErasureReceipt {
            user_id: user_id.to_owned(),
            sessions_revoked,
            deliveries_removed: before_deliveries - self.deliveries.len(),
            deadlines_removed: before_deadlines - self.deadlines.len(),
            report_rows_removed: before_reports - self.report_rows.len(),
            credential_revoked: true,
        }
    }
}

pub async fn erase_account(
    client: &InfraiClient,
    records: &mut LearningRecords,
    user_id: &str,
    credential_id: &str,
) -> Result<ErasureReceipt, ErasureError> {
    if client.control_credential_id() == Some(credential_id) {
        return Err(ErasureError::UnsafeCredentialTarget);
    }

    let sessions = client.list_sessions(user_id).await?;
    for session in &sessions {
        client.revoke_session(&session.id).await?;
    }
    client.revoke_key(credential_id).await?;

    Ok(records.erase_learner(user_id, sessions.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasure_removes_only_the_selected_learners_operational_records() {
        let mut records = LearningRecords {
            deliveries: vec![
                CourseDelivery {
                    learner_id: "learner-42".into(),
                    course_id: "rust-101".into(),
                },
                CourseDelivery {
                    learner_id: "learner-7".into(),
                    course_id: "math-201".into(),
                },
            ],
            deadlines: vec![LearnerDeadline {
                learner_id: "learner-42".into(),
                course_id: "rust-101".into(),
                due_at: "2026-10-15T09:00:00Z".into(),
            }],
            report_rows: vec![EducatorReportRow {
                learner_id: "learner-42".into(),
                course_id: "rust-101".into(),
                completion_percent: 80,
            }],
        };

        let receipt = records.erase_learner("learner-42", 2);

        assert_eq!(receipt.sessions_revoked, 2);
        assert_eq!(receipt.deliveries_removed, 1);
        assert_eq!(receipt.deadlines_removed, 1);
        assert_eq!(receipt.report_rows_removed, 1);
        assert_eq!(records.deliveries[0].learner_id, "learner-7");
        assert!(records.deadlines.is_empty());
        assert!(records.report_rows.is_empty());
    }
}
