use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
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
    submissions_per_ip: HashMap<String, Vec<DateTime<Utc>>>,
}

impl SubmissionQueue {
    pub fn new() -> Self {
        Self {
            submissions: Vec::new(),
            submissions_per_ip: HashMap::new(),
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
        // Backwards-compatible fallback for call sites that do not yet supply source IP.
        // Use contributor ID as a stable per-client key.
        self.submit_with_ip(
            contributor_id,
            submission_type,
            primary_source_url,
            primary_source_type,
            reference_detail,
            description,
            contributor_id.0.to_string(),
        )
    }

    /// Submit a new connection/entity for review with source IP rate limiting
    pub fn submit_with_ip(
        &mut self,
        contributor_id: ContributorId,
        mut submission_type: SubmissionType,
        primary_source_url: String,
        primary_source_type: SourceType,
        reference_detail: String,
        description: String,
        source_ip: String,
    ) -> CrowdResult<SubmissionId> {
        let primary_source_url = sanitize_text(primary_source_url);
        let reference_detail = sanitize_text(reference_detail);
        let description = sanitize_text(description);
        let source_ip = sanitize_text(source_ip);

        validate_required_field("primary source URL", &primary_source_url)?;
        validate_required_field("reference detail (quote or page number)", &reference_detail)?;
        validate_required_field("description", &description)?;
        validate_required_field("source IP", &source_ip)?;
        validate_url("primary source URL", &primary_source_url)?;
        validate_submission_type(&mut submission_type)?;

        self.check_ip_rate_limit(&source_ip)?;

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

        validate_status_transition(sub.status, SubmissionStatus::InReview)?;

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

        validate_status_transition(sub.status, SubmissionStatus::Approved)?;

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

        validate_status_transition(sub.status, SubmissionStatus::Rejected)?;

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

        validate_status_transition(sub.status, SubmissionStatus::Disputed)?;

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

    fn check_ip_rate_limit(&mut self, source_ip: &str) -> CrowdResult<()> {
        const MAX_SUBMISSIONS_PER_IP_PER_HOUR: usize = 10;
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        let recent_submissions = self
            .submissions_per_ip
            .entry(source_ip.to_string())
            .or_default();
        recent_submissions.retain(|ts| *ts > one_hour_ago);

        if recent_submissions.len() >= MAX_SUBMISSIONS_PER_IP_PER_HOUR {
            return Err(CrowdError::RateLimited(format!(
                "max {MAX_SUBMISSIONS_PER_IP_PER_HOUR} submissions per IP per hour"
            )));
        }

        recent_submissions.push(now);
        Ok(())
    }
}

fn validate_required_field(field_name: &str, value: &str) -> CrowdResult<()> {
    if value.is_empty() {
        return Err(CrowdError::InvalidSource(format!(
            "{field_name} is required"
        )));
    }

    Ok(())
}

fn validate_url(field_name: &str, value: &str) -> CrowdResult<()> {
    let parsed = Url::parse(value)
        .map_err(|_| CrowdError::InvalidSource(format!("{field_name} must be a valid URL")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CrowdError::InvalidSource(format!(
            "{field_name} must use http or https"
        )));
    }

    if parsed.host_str().is_none() {
        return Err(CrowdError::InvalidSource(format!(
            "{field_name} must include a host"
        )));
    }

    Ok(())
}

fn validate_status_transition(
    current: SubmissionStatus,
    target: SubmissionStatus,
) -> CrowdResult<()> {
    let valid = matches!(
        (current, target),
        (SubmissionStatus::Pending, SubmissionStatus::InReview)
            | (SubmissionStatus::InReview, SubmissionStatus::Approved)
            | (SubmissionStatus::InReview, SubmissionStatus::Rejected)
            | (SubmissionStatus::Rejected, SubmissionStatus::Disputed)
    );

    if valid {
        return Ok(());
    }

    let reason = if target == SubmissionStatus::Disputed {
        "only rejected submissions can be disputed".to_string()
    } else {
        format!("invalid status transition: {current:?} -> {target:?}")
    };

    Err(CrowdError::Rejected { reason })
}

fn validate_submission_type(submission_type: &mut SubmissionType) -> CrowdResult<()> {
    match submission_type {
        SubmissionType::NewConnection { .. } => {}
        SubmissionType::NewEntity { entity_type, .. } => {
            *entity_type = sanitize_text(entity_type.clone());
            validate_required_field("entity type", entity_type)?;
        }
        SubmissionType::Correction {
            field,
            current_value,
            proposed_value,
            ..
        } => {
            *field = sanitize_text(field.clone());
            *current_value = sanitize_text(current_value.clone());
            *proposed_value = sanitize_text(proposed_value.clone());
            validate_required_field("correction field", field)?;
            validate_required_field("current value", current_value)?;
            validate_required_field("proposed value", proposed_value)?;
        }
        SubmissionType::ConductComparison {
            official_action,
            equivalent_private_conduct,
            documented_consequence,
            consequence_source_url,
            ..
        } => {
            *official_action = sanitize_text(official_action.clone());
            *equivalent_private_conduct = sanitize_text(equivalent_private_conduct.clone());
            *documented_consequence = sanitize_text(documented_consequence.clone());
            *consequence_source_url = sanitize_text(consequence_source_url.clone());
            validate_required_field("official action", official_action)?;
            validate_required_field("equivalent private conduct", equivalent_private_conduct)?;
            validate_required_field("documented consequence", documented_consequence)?;
            validate_required_field("consequence source URL", consequence_source_url)?;
            validate_url("consequence source URL", consequence_source_url)?;
        }
    }

    Ok(())
}

fn sanitize_text(input: String) -> String {
    let without_control_chars: String = input
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect();

    without_control_chars
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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
    fn test_submit_requires_description() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "https://example.gov/filing".to_string(),
            SourceType::FecFiling,
            "ref".to_string(),
            "   ".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_rejects_invalid_primary_source_url() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "notaurl".to_string(),
            SourceType::FecFiling,
            "ref".to_string(),
            "desc".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_rejects_non_http_source_url() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "ftp://example.gov/filing".to_string(),
            SourceType::FecFiling,
            "ref".to_string(),
            "desc".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_sanitizes_text_fields() {
        let mut queue = SubmissionQueue::new();
        let contributor = ContributorId::new();

        let id = queue
            .submit(
                contributor,
                SubmissionType::NewEntity {
                    entity_type: " Person \n".to_string(),
                    entity_data: serde_json::json!({"name": "Test"}),
                },
                "  https://example.gov/filing  ".to_string(),
                SourceType::FecFiling,
                " line 12 \n\n quote ".to_string(),
                "  some\t\t description \n here ".to_string(),
            )
            .unwrap();

        let stored = queue.get(id).unwrap();
        assert_eq!(stored.primary_source_url, "https://example.gov/filing");
        assert_eq!(stored.reference_detail, "line 12 quote");
        assert_eq!(stored.description, "some description here");

        match &stored.submission_type {
            SubmissionType::NewEntity { entity_type, .. } => assert_eq!(entity_type, "Person"),
            _ => panic!("expected NewEntity"),
        }
    }

    #[test]
    fn test_submit_validates_conduct_comparison_url() {
        let mut queue = SubmissionQueue::new();
        let result = queue.submit(
            ContributorId::new(),
            SubmissionType::ConductComparison {
                official_action: "Action".to_string(),
                official_id: EntityId::new(),
                equivalent_private_conduct: "Conduct".to_string(),
                documented_consequence: "Consequence".to_string(),
                consequence_source_url: "javascript:alert(1)".to_string(),
            },
            "https://example.gov/filing".to_string(),
            SourceType::CourtRecord,
            "ref".to_string(),
            "desc".to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_submit_with_ip_rate_limited_after_ten() {
        let mut queue = SubmissionQueue::new();
        let contributor = ContributorId::new();
        let ip = "203.0.113.5".to_string();

        for _ in 0..10 {
            queue
                .submit_with_ip(
                    contributor,
                    SubmissionType::NewEntity {
                        entity_type: "Person".to_string(),
                        entity_data: serde_json::json!({"name": "Test"}),
                    },
                    "https://example.gov/filing".to_string(),
                    SourceType::FecFiling,
                    "ref".to_string(),
                    "desc".to_string(),
                    ip.clone(),
                )
                .unwrap();
        }

        let result = queue.submit_with_ip(
            contributor,
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "https://example.gov/filing-2".to_string(),
            SourceType::FecFiling,
            "ref".to_string(),
            "desc".to_string(),
            ip,
        );

        assert!(matches!(result, Err(CrowdError::RateLimited(_))));
    }

    #[test]
    fn test_submit_with_ip_rate_limit_is_per_ip() {
        let mut queue = SubmissionQueue::new();
        let contributor = ContributorId::new();

        for _ in 0..10 {
            queue
                .submit_with_ip(
                    contributor,
                    SubmissionType::NewEntity {
                        entity_type: "Person".to_string(),
                        entity_data: serde_json::json!({"name": "Test"}),
                    },
                    "https://example.gov/filing".to_string(),
                    SourceType::FecFiling,
                    "ref".to_string(),
                    "desc".to_string(),
                    "198.51.100.10".to_string(),
                )
                .unwrap();
        }

        let result = queue.submit_with_ip(
            contributor,
            SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            "https://example.gov/filing".to_string(),
            SourceType::FecFiling,
            "ref".to_string(),
            "desc".to_string(),
            "198.51.100.11".to_string(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_full_submission_workflow() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();

        assert_eq!(queue.count_by_status(SubmissionStatus::Pending), 1);

        // Claim
        queue.claim_for_review(sub_id, reviewer).unwrap();
        assert_eq!(
            queue.get(sub_id).unwrap().status,
            SubmissionStatus::InReview
        );

        // Approve
        queue.approve(sub_id, reviewer).unwrap();
        assert_eq!(
            queue.get(sub_id).unwrap().status,
            SubmissionStatus::Approved
        );
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
        assert!(
            sub.review_note
                .as_ref()
                .unwrap()
                .contains("NotPrimarySource")
        );
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

        assert_eq!(
            queue.get(sub_id).unwrap().status,
            SubmissionStatus::Disputed
        );
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
    fn test_cannot_reject_pending() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();
        let result = queue.reject(
            sub_id,
            reviewer,
            RejectionReason::Incomplete,
            "not enough detail".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_claim_approved() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();
        queue.claim_for_review(sub_id, reviewer).unwrap();
        queue.approve(sub_id, reviewer).unwrap();

        let result = queue.claim_for_review(sub_id, reviewer);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_approve_rejected() {
        let (mut queue, _contributor, sub_id) = make_queue_with_submission();
        let reviewer = ContributorId::new();
        queue.claim_for_review(sub_id, reviewer).unwrap();
        queue
            .reject(
                sub_id,
                reviewer,
                RejectionReason::SourceDoesNotSupport,
                "not supported".to_string(),
            )
            .unwrap();

        let result = queue.approve(sub_id, reviewer);
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
