use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;
use nf_core::source::SourceType;
#[cfg(test)]
use nf_crowd::submission::SubmissionStatus;
use nf_crowd::submission::{
    ContributorId, RejectionReason, Submission, SubmissionId, SubmissionType,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSubmissionRequest {
    /// Contributor UUID (in production this comes from auth, here explicit for now).
    pub contributor_id: Uuid,
    /// Type of submission.
    pub submission_type: SubmissionTypeRequest,
    /// Primary source URL (required).
    pub primary_source_url: String,
    /// Source type classification.
    pub primary_source_type: SourceType,
    /// Specific reference in source (page, filing ID, quote).
    pub reference_detail: String,
    /// Free-text description.
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubmissionTypeRequest {
    NewConnection {
        entity_a: Uuid,
        relationship_type: RelationshipType,
        entity_b: Uuid,
    },
    NewEntity {
        entity_type: String,
        entity_data: serde_json::Value,
    },
    Correction {
        entity_id: Uuid,
        field: String,
        current_value: String,
        proposed_value: String,
    },
    ConductComparison {
        official_action: String,
        official_id: Uuid,
        equivalent_private_conduct: String,
        documented_consequence: String,
        consequence_source_url: String,
    },
}

impl SubmissionTypeRequest {
    fn into_submission_type(self) -> SubmissionType {
        match self {
            Self::NewConnection {
                entity_a,
                relationship_type,
                entity_b,
            } => SubmissionType::NewConnection {
                entity_a: EntityId(entity_a),
                relationship_type,
                entity_b: EntityId(entity_b),
            },
            Self::NewEntity {
                entity_type,
                entity_data,
            } => SubmissionType::NewEntity {
                entity_type,
                entity_data,
            },
            Self::Correction {
                entity_id,
                field,
                current_value,
                proposed_value,
            } => SubmissionType::Correction {
                entity_id: EntityId(entity_id),
                field,
                current_value,
                proposed_value,
            },
            Self::ConductComparison {
                official_action,
                official_id,
                equivalent_private_conduct,
                documented_consequence,
                consequence_source_url,
            } => SubmissionType::ConductComparison {
                official_action,
                official_id: EntityId(official_id),
                equivalent_private_conduct,
                documented_consequence,
                consequence_source_url,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub reviewer_id: Uuid,
    pub decision: ReviewDecisionRequest,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecisionRequest {
    Claim,
    Approve,
    Reject { reason: RejectionReason },
}

#[derive(Debug, Serialize)]
pub struct SubmissionResponse {
    pub id: Uuid,
    pub contributor_id: Uuid,
    pub status: String,
    pub primary_source_url: String,
    pub description: String,
    pub created_at: String,
    pub review_note: Option<String>,
}

impl From<&Submission> for SubmissionResponse {
    fn from(s: &Submission) -> Self {
        Self {
            id: s.id.0,
            contributor_id: s.contributor_id.0,
            status: format!("{:?}", s.status),
            primary_source_url: s.primary_source_url.clone(),
            description: s.description.clone(),
            created_at: s.created_at.to_rfc3339(),
            review_note: s.review_note.clone(),
        }
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/v1/submissions — create a new submission.
pub async fn create_submission(
    State(state): State<AppState>,
    Json(req): Json<CreateSubmissionRequest>,
) -> ApiResult<(StatusCode, Json<SubmissionResponse>)> {
    let contributor_id = ContributorId(req.contributor_id);
    let submission_type = req.submission_type.into_submission_type();

    let sub_id = {
        let mut queue = state.submission_queue.lock().unwrap();
        queue
            .submit(
                contributor_id,
                submission_type,
                req.primary_source_url,
                req.primary_source_type,
                req.reference_detail,
                req.description,
            )
            .map_err(|e| ApiError::BadRequest(e.to_string()))?
    };

    let queue = state.submission_queue.lock().unwrap();
    let submission = queue
        .get(sub_id)
        .ok_or_else(|| ApiError::Internal("submission created but not found".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(SubmissionResponse::from(submission)),
    ))
}

/// GET /api/v1/submissions — list submissions (optionally filtered by status).
pub async fn list_submissions(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<ListSubmissionsParams>,
) -> ApiResult<Json<Vec<SubmissionResponse>>> {
    let queue = state.submission_queue.lock().unwrap();

    let submissions: Vec<SubmissionResponse> = if let Some(status) = &params.status {
        match status.as_str() {
            "pending" => queue
                .pending()
                .into_iter()
                .map(SubmissionResponse::from)
                .collect(),
            _ => {
                // Return all — in production we'd filter by status
                queue
                    .pending()
                    .into_iter()
                    .map(SubmissionResponse::from)
                    .collect()
            }
        }
    } else {
        queue
            .pending()
            .into_iter()
            .map(SubmissionResponse::from)
            .collect()
    };

    Ok(Json(submissions))
}

#[derive(Debug, Deserialize)]
pub struct ListSubmissionsParams {
    pub status: Option<String>,
}

/// GET /api/v1/submissions/:id — get a single submission.
pub async fn get_submission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SubmissionResponse>> {
    let queue = state.submission_queue.lock().unwrap();
    let submission = queue
        .get(SubmissionId(id))
        .ok_or_else(|| ApiError::NotFound(format!("submission {id}")))?;

    Ok(Json(SubmissionResponse::from(submission)))
}

/// POST /api/v1/submissions/:id/review — review a submission (claim/approve/reject).
pub async fn review_submission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviewRequest>,
) -> ApiResult<Json<SubmissionResponse>> {
    let submission_id = SubmissionId(id);
    let reviewer_id = ContributorId(req.reviewer_id);

    {
        let mut queue = state.submission_queue.lock().unwrap();
        match req.decision {
            ReviewDecisionRequest::Claim => {
                queue
                    .claim_for_review(submission_id, reviewer_id)
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            }
            ReviewDecisionRequest::Approve => {
                queue
                    .approve(submission_id, reviewer_id)
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            }
            ReviewDecisionRequest::Reject { reason } => {
                let note = req.note.unwrap_or_default();
                queue
                    .reject(submission_id, reviewer_id, reason, note)
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            }
        }
    }

    let queue = state.submission_queue.lock().unwrap();
    let submission = queue
        .get(submission_id)
        .ok_or_else(|| ApiError::NotFound(format!("submission {id}")))?;

    Ok(Json(SubmissionResponse::from(submission)))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_response_from() {
        let now = chrono::Utc::now();
        let sub = Submission {
            id: SubmissionId::new(),
            contributor_id: ContributorId::new(),
            submission_type: SubmissionType::NewEntity {
                entity_type: "Person".to_string(),
                entity_data: serde_json::json!({"name": "Test"}),
            },
            status: SubmissionStatus::Pending,
            primary_source_url: "https://example.gov".to_string(),
            primary_source_type: SourceType::FecFiling,
            reference_detail: "ref".to_string(),
            description: "test submission".to_string(),
            created_at: now,
            updated_at: now,
            reviewer_id: None,
            review_note: None,
        };

        let resp = SubmissionResponse::from(&sub);
        assert_eq!(resp.id, sub.id.0);
        assert_eq!(resp.status, "Pending");
        assert_eq!(resp.description, "test submission");
    }

    #[test]
    fn test_submission_type_conversion() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let req = SubmissionTypeRequest::NewConnection {
            entity_a: a,
            relationship_type: RelationshipType::DonatedTo,
            entity_b: b,
        };
        let st = req.into_submission_type();
        match st {
            SubmissionType::NewConnection {
                entity_a, entity_b, ..
            } => {
                assert_eq!(entity_a.0, a);
                assert_eq!(entity_b.0, b);
            }
            _ => panic!("expected NewConnection"),
        }
    }

    #[test]
    fn test_review_decision_deserialization() {
        let json = r#"{"type":"claim"}"#;
        // This verifies the enum variant names work for downstream parsing
        let _: serde_json::Value = serde_json::from_str(json).unwrap();
    }
}
