use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::submission::{ContributorId, RejectionReason, SubmissionId, SubmissionStatus};

/// Every moderation decision is logged publicly — full audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAction {
    pub id: Uuid,
    pub submission_id: SubmissionId,
    pub reviewer_id: ContributorId,
    pub action: ReviewDecision,
    pub timestamp: DateTime<Utc>,
    /// Public rationale for the decision
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewDecision {
    Claimed,
    Approved,
    Rejected(RejectionReason),
    DisputeOpened,
    DisputeResolved(SubmissionStatus),
}

/// Audit trail of all review actions — logged publicly
#[derive(Debug)]
pub struct ReviewLog {
    actions: Vec<ReviewAction>,
}

impl ReviewLog {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn log_action(
        &mut self,
        submission_id: SubmissionId,
        reviewer_id: ContributorId,
        action: ReviewDecision,
        rationale: String,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.actions.push(ReviewAction {
            id,
            submission_id,
            reviewer_id,
            action,
            timestamp: Utc::now(),
            rationale,
        });
        id
    }

    pub fn actions_for_submission(&self, submission_id: SubmissionId) -> Vec<&ReviewAction> {
        self.actions
            .iter()
            .filter(|a| a.submission_id == submission_id)
            .collect()
    }

    pub fn actions_by_reviewer(&self, reviewer_id: ContributorId) -> Vec<&ReviewAction> {
        self.actions
            .iter()
            .filter(|a| a.reviewer_id == reviewer_id)
            .collect()
    }

    pub fn total_actions(&self) -> usize {
        self.actions.len()
    }

    /// Export all actions for public audit
    pub fn export_all(&self) -> &[ReviewAction] {
        &self.actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_and_retrieve() {
        let mut log = ReviewLog::new();
        let sub_id = SubmissionId::new();
        let reviewer = ContributorId::new();

        log.log_action(
            sub_id,
            reviewer,
            ReviewDecision::Claimed,
            "Starting review".to_string(),
        );
        log.log_action(
            sub_id,
            reviewer,
            ReviewDecision::Approved,
            "Source verified against FEC API".to_string(),
        );

        let actions = log.actions_for_submission(sub_id);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, ReviewDecision::Claimed);
        assert_eq!(actions[1].action, ReviewDecision::Approved);
    }

    #[test]
    fn test_actions_by_reviewer() {
        let mut log = ReviewLog::new();
        let reviewer = ContributorId::new();

        log.log_action(
            SubmissionId::new(),
            reviewer,
            ReviewDecision::Approved,
            "good".to_string(),
        );
        log.log_action(
            SubmissionId::new(),
            reviewer,
            ReviewDecision::Rejected(RejectionReason::NotPrimarySource),
            "news article not government record".to_string(),
        );

        assert_eq!(log.actions_by_reviewer(reviewer).len(), 2);
    }

    #[test]
    fn test_export_all() {
        let mut log = ReviewLog::new();
        log.log_action(
            SubmissionId::new(),
            ContributorId::new(),
            ReviewDecision::Approved,
            "test".to_string(),
        );
        assert_eq!(log.export_all().len(), 1);
    }
}
