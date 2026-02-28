use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;
use nf_core::source::SourceType;

use crate::error::{CrowdError, CrowdResult};

/// A submission from a contributor proposing a new connection or entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: SubmissionId,
    pub contributor_id: ContributorId,
    pub submission_type: SubmissionType,
    pub status: SubmissionStatus,
    /// The primary source document (required)
    pub primary_source_url: String,
    /// Type of primary source
    pub primary_source_type: SourceType,
    /// Direct quote or specific page/filing reference (required)
    pub reference_detail: String,
    /// Free-text description of the connection
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Reviewer who handled this submission
    pub reviewer_id: Option<ContributorId>,
    /// Reason for rejection/dispute (if applicable)
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SubmissionId(pub Uuid);

impl SubmissionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContributorId(pub Uuid);

impl ContributorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmissionType {
    /// New connection between two existing entities
    NewConnection {
        entity_a: EntityId,
        relationship_type: RelationshipType,
        entity_b: EntityId,
    },
    /// New entity not yet in the database
    NewEntity {
        entity_type: String,
        entity_data: serde_json::Value,
    },
    /// Correction to an existing entity
    Correction {
        entity_id: EntityId,
        field: String,
        current_value: String,
        proposed_value: String,
    },
    /// New conduct comparison entry
    ConductComparison {
        official_action: String,
        official_id: EntityId,
        equivalent_private_conduct: String,
        documented_consequence: String,
        consequence_source_url: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubmissionStatus {
    /// Awaiting review
    Pending,
    /// Under active review by a maintainer
    InReview,
    /// Verified and published
    Approved,
    /// Rejected with reason
    Rejected,
    /// Disputed — flagged publicly with evidence from both sides
    Disputed,
    /// Withdrawn by submitter
    Withdrawn,
}

/// Rejection criteria — every rejection must cite one of these
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RejectionReason {
    /// Source is not a primary government document
    NotPrimarySource,
    /// Source does not support the stated connection
    SourceDoesNotSupport,
    /// Duplicate of existing verified entry
    Duplicate,
    /// Contains personal information of private individuals
    PrivateInformation,
    /// Unverified allegations without documentary basis
    UnverifiedAllegation,
    /// Incorrect entity identification
    WrongEntity,
    /// Formatting or completeness issue
    Incomplete,
}

/// The submission queue manages the flow from submission to publication
#[derive(Debug)]
pub struct SubmissionQueue {
    submissions: Vec<Submission>,
}

impl SubmissionQueue {
    pub fn new() -> Self {
        Self {
            submissions: Vec::new(),
        }
    }

    /// Submit a new connection/entity for review
    pub fn submit(
        &mut self,
        contributor_id: ContributorId,
        submission_type: SubmissionType,
        primary_source_url: String,
        primary_source_type: SourceType,
        reference_detail: String,
        description: String,
    ) -> CrowdResult<SubmissionId> {
        // Validate required fields
        if primary_source_url.is_empty() {
            return Err(CrowdError::InvalidSource(
                "primary source URL is required".to_string(),
            ));
        }
        if reference_detail.is_empty() {
            return Err(CrowdError::InvalidSource(
                "reference detail (quote or page number) is required".to_string(),
            ));
        }

        let id = SubmissionId::new();
        let now = Utc::now();
        let submission = Submission {
            id,
            contributor_id,
            submission_type,
            status: SubmissionStatus::Pending,
            primary_source_url,
            primary_source_type,
            reference_detail,
            description,
            created_at: now,
            updated_at: now,
            reviewer_id: None,
            review_note: None,
        };

        self.submissions.push(submission);
        Ok(id)
    }

    /// Get all pending submissions (FIFO order)
    pub fn pending(&self) -> Vec<&Submission> {
        self.submissions
            .iter()
            .filter(|s| s.status == SubmissionStatus::Pending)
            .collect()
    }

    /// Get submissions by contributor
    pub fn by_contributor(&self, contributor_id: ContributorId) -> Vec<&Submission> {
        self.submissions
            .iter()
            .filter(|s| s.contributor_id == contributor_id)
            .collect()
    }

    /// Claim a submission for review
    pub fn claim_for_review(
        &mut self,
        submission_id: SubmissionId,
        reviewer_id: ContributorId,
    ) -> CrowdResult<()> {
        let sub = self
            .submissions
            .iter_mut()
            .find(|s| s.id == submission_id)
            .ok_or_else(|| CrowdError::NotFound(format!("submission {}", submission_id.0)))?;

        if sub.status != SubmissionStatus::Pending {
            return Err(CrowdError::Rejected {
                reason: format!("submission is {:?}, not Pending", sub.status),
            });
        }

        sub.status = SubmissionStatus::InReview;
        sub.reviewer_id = Some(reviewer_id);
        sub.updated_at = Utc::now();
        Ok(())
    }

    /// Approve a submission
    pub fn approve(
        &mut self,
        submission_id: SubmissionId,
        reviewer_id: ContributorId,
    ) -> CrowdResult<()> {
        let sub = self
            .submissions
            .iter_mut()
            .find(|s| s.id == submission_id)
            .ok_or_else(|| CrowdError::NotFound(format!("submission {}", submission_id.0)))?;

        if sub.status != SubmissionStatus::InReview {
            return Err(CrowdError::Rejected {
                reason: "submission must be InReview to approve".to_string(),
            });
        }

        sub.status = SubmissionStatus::Approved;
        sub.reviewer_id = Some(reviewer_id);
        sub.updated_at = Utc::now();
        Ok(())
    }

    /// Reject a submission with a specific reason
    pub fn reject(
        &mut self,
        submission_id: SubmissionId,
        reviewer_id: ContributorId,
        reason: RejectionReason,
        note: String,
    ) -> CrowdResult<()> {
        let sub = self
            .submissions
            .iter_mut()
            .find(|s| s.id == submission_id)
            .ok_or_else(|| CrowdError::NotFound(format!("submission {}", submission_id.0)))?;

        if sub.status != SubmissionStatus::InReview {
            return Err(CrowdError::Rejected {
                reason: "submission must be InReview to reject".to_string(),
            });
        }

        sub.status = SubmissionStatus::Rejected;
        sub.reviewer_id = Some(reviewer_id);
        sub.review_note = Some(format!("{reason:?}: {note}"));
        sub.updated_at = Utc::now();
        Ok(())
    }

    /// Dispute a rejected submission
    pub fn dispute(
        &mut self,
        submission_id: SubmissionId,
        dispute_evidence: String,
    ) -> CrowdResult<()> {
        let sub = self
            .submissions
            .iter_mut()
            .find(|s| s.id == submission_id)
            .ok_or_else(|| CrowdError::NotFound(format!("submission {}", submission_id.0)))?;

        if sub.status != SubmissionStatus::Rejected {
            return Err(CrowdError::Rejected {
                reason: "only rejected submissions can be disputed".to_string(),
            });
        }

        sub.status = SubmissionStatus::Disputed;
        sub.review_note = Some(format!(
            "{}\n\nDISPUTE: {}",
            sub.review_note.as_deref().unwrap_or(""),
            dispute_evidence
        ));
        sub.updated_at = Utc::now();
        Ok(())
    }

    /// Get a submission by ID
    pub fn get(&self, id: SubmissionId) -> Option<&Submission> {
        self.submissions.iter().find(|s| s.id == id)
    }

    /// Total count by status
    pub fn count_by_status(&self, status: SubmissionStatus) -> usize {
        self.submissions
            .iter()
            .filter(|s| s.status == status)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::source::SourceType;

    fn make_queue_with_submission() -> (SubmissionQueue, ContributorId, SubmissionId) {
        let mut queue = SubmissionQueue::new();
        let contributor = ContributorId::new();
        let entity_a = EntityId::new();
        let entity_b = EntityId::new();

        let id = queue
            .submit(
                contributor,
                SubmissionType::NewConnection {
                    entity_a,
                    relationship_type: RelationshipType::DonatedTo,
                    entity_b,
                },
                "https://api.open.fec.gov/v1/schedules/schedule_a/?committee_id=C00001234"
                    .to_string(),
                SourceType::FecFiling,
                "Filing ID: FEC-2024-001, Line 12A, $500,000 contribution".to_string(),
                "Donation from PAC to campaign committee".to_string(),
            )
            .unwrap();

        (queue, contributor, id)
    }

    #[test]
    fn test_submit_requires_source_url() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "".to_string(), // empty URL
            SourceType::FecFiling,
            "some ref".to_string(),
            "desc".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_requires_reference_detail() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "https://example.gov/filing".to_string(),
            SourceType::FecFiling,
            "".to_string(), // empty reference
            "desc".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_full_submission_workflow() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();

        assert_eq!(queue.count_by_status(SubmissionStatus::Pending), 1);

        // Claim
        queue.claim_for_review(sub_id, reviewer).unwrap();
        assert_eq!(queue.get(sub_id).unwrap().status, SubmissionStatus::InReview);

        // Approve
        queue.approve(sub_id, reviewer).unwrap();
        assert_eq!(queue.get(sub_id).unwrap().status, SubmissionStatus::Approved);
        assert_eq!(queue.count_by_status(SubmissionStatus::Approved), 1);
    }

    #[test]
    fn test_rejection_workflow() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();

        queue.claim_for_review(sub_id, reviewer).unwrap();
        queue
            .reject(
                sub_id,
                reviewer,
                RejectionReason::NotPrimarySource,
                "Source is a news article, not a government record".to_string(),
            )
            .unwrap();

        let sub = queue.get(sub_id).unwrap();
        assert_eq!(sub.status, SubmissionStatus::Rejected);
        assert!(sub.review_note.as_ref().unwrap().contains("NotPrimarySource"));
    }

    #[test]
    fn test_dispute_workflow() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();

        queue.claim_for_review(sub_id, reviewer).unwrap();
        queue
            .reject(
                sub_id,
                reviewer,
                RejectionReason::SourceDoesNotSupport,
                "Filing doesn't show this connection".to_string(),
            )
            .unwrap();

        queue
            .dispute(
                sub_id,
                "See page 3 of the filing — the connection is in the Schedule A itemization"
                    .to_string(),
            )
            .unwrap();

        assert_eq!(queue.get(sub_id).unwrap().status, SubmissionStatus::Disputed);
    }

    #[test]
    fn test_cannot_approve_pending() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let result = queue.approve(sub_id, ContributorId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_dispute_approved() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();
        queue.claim_for_review(sub_id, reviewer).unwrap();
        queue.approve(sub_id, reviewer).unwrap();
        let result = queue.dispute(sub_id, "evidence".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_pending_returns_fifo() {
        let mut queue = SubmissionQueue::new();
        let c = ContributorId::new();
        let a = EntityId::new();
        let b = EntityId::new();

        let id1 = queue
            .submit(
                c,
                SubmissionType::NewConnection {
                    entity_a: a,
                    relationship_type: RelationshipType::DonatedTo,
                    entity_b: b,
                },
                "https://fec.gov/1".to_string(),
                SourceType::FecFiling,
                "ref1".to_string(),
                "first".to_string(),
            )
            .unwrap();

        let id2 = queue
            .submit(
                c,
                SubmissionType::NewConnection {
                    entity_a: b,
                    relationship_type: RelationshipType::NamedIn,
                    entity_b: a,
                },
                "https://pacer.gov/1".to_string(),
                SourceType::CourtRecord,
                "ref2".to_string(),
                "second".to_string(),
            )
            .unwrap();

        let pending = queue.pending();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].id, id1);
        assert_eq!(pending[1].id, id2);
    }
}
