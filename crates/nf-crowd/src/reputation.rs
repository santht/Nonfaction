use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::submission::{ContributorId, SubmissionId, SubmissionStatus};

/// Reputation system — verified submissions build score, bad faith submissions deduct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorProfile {
    pub id: ContributorId,
    pub display_name: String,
    pub email_hash: String, // SHA-256 of email for Gravatar, never store raw
    pub reputation_score: i64,
    pub total_submissions: u64,
    pub approved_submissions: u64,
    pub rejected_submissions: u64,
    pub disputed_submissions: u64,
    pub trust_tier: TrustTier,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
    /// Rate limit: max submissions per hour for this contributor
    pub submissions_per_hour: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustTier {
    /// New contributor — all submissions go through full review
    New,
    /// Established — some submissions get expedited review
    Established,
    /// Trusted — most submissions get expedited review
    Trusted,
    /// Verified — organization-verified contributor (journalist, researcher)
    Verified,
    /// Maintainer — can review and approve submissions
    Maintainer,
}

impl TrustTier {
    pub fn submissions_per_hour(&self) -> u32 {
        match self {
            TrustTier::New => 5,
            TrustTier::Established => 15,
            TrustTier::Trusted => 30,
            TrustTier::Verified => 60,
            TrustTier::Maintainer => 120,
        }
    }

    pub fn expedited_review(&self) -> bool {
        matches!(
            self,
            TrustTier::Trusted | TrustTier::Verified | TrustTier::Maintainer
        )
    }
}

/// Points awarded/deducted for various actions
const POINTS_APPROVED: i64 = 10;
const POINTS_REJECTED: i64 = -3;
const POINTS_DISPUTED_WON: i64 = 15; // wrongful rejection overturned
const POINTS_BAD_FAITH: i64 = -25;

/// Thresholds for tier promotion
const TIER_ESTABLISHED: i64 = 50;
const TIER_TRUSTED: i64 = 200;

/// Reputation tracker for all contributors
#[derive(Debug)]
pub struct ReputationTracker {
    profiles: HashMap<ContributorId, ContributorProfile>,
    /// Sybil detection: track submission patterns
    recent_submissions: HashMap<ContributorId, Vec<DateTime<Utc>>>,
}

impl ReputationTracker {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            recent_submissions: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        display_name: impl Into<String>,
        email_hash: impl Into<String>,
    ) -> ContributorId {
        let id = ContributorId::new();
        let now = Utc::now();
        let profile = ContributorProfile {
            id,
            display_name: display_name.into(),
            email_hash: email_hash.into(),
            reputation_score: 0,
            total_submissions: 0,
            approved_submissions: 0,
            rejected_submissions: 0,
            disputed_submissions: 0,
            trust_tier: TrustTier::New,
            joined_at: now,
            last_active: now,
            suspended: false,
            suspension_reason: None,
            submissions_per_hour: TrustTier::New.submissions_per_hour(),
        };
        self.profiles.insert(id, profile);
        id
    }

    pub fn get_profile(&self, id: ContributorId) -> Option<&ContributorProfile> {
        self.profiles.get(&id)
    }

    /// Record a submission result and update reputation
    pub fn record_result(
        &mut self,
        contributor_id: ContributorId,
        _submission_id: SubmissionId,
        status: SubmissionStatus,
    ) {
        if let Some(profile) = self.profiles.get_mut(&contributor_id) {
            profile.total_submissions += 1;
            profile.last_active = Utc::now();

            match status {
                SubmissionStatus::Approved => {
                    profile.approved_submissions += 1;
                    profile.reputation_score += POINTS_APPROVED;
                }
                SubmissionStatus::Rejected => {
                    profile.rejected_submissions += 1;
                    profile.reputation_score += POINTS_REJECTED;
                }
                SubmissionStatus::Disputed => {
                    profile.disputed_submissions += 1;
                    profile.reputation_score += POINTS_DISPUTED_WON;
                }
                _ => {}
            }

            // Auto-suspend for bad faith (3+ rejections with score < -10, no approvals)
            if profile.reputation_score < -10
                && profile.rejected_submissions >= 3
                && profile.approved_submissions == 0
            {
                profile.suspended = true;
                profile.suspension_reason = Some(
                    "Automatic suspension: repeated rejected submissions with no approvals"
                        .to_string(),
                );
            }
        }

        // Auto-promote based on score (separate borrow scope)
        self.update_tier(contributor_id);
    }

    /// Record a bad-faith submission (spam, intentionally misleading)
    pub fn record_bad_faith(&mut self, contributor_id: ContributorId, reason: String) {
        if let Some(profile) = self.profiles.get_mut(&contributor_id) {
            profile.reputation_score += POINTS_BAD_FAITH;
            profile.suspended = true;
            profile.suspension_reason = Some(format!("Bad faith: {reason}"));
        }
    }

    fn update_tier(&mut self, contributor_id: ContributorId) {
        if let Some(profile) = self.profiles.get_mut(&contributor_id) {
            // Don't demote maintainers or verified contributors
            if matches!(
                profile.trust_tier,
                TrustTier::Maintainer | TrustTier::Verified
            ) {
                return;
            }

            let new_tier = if profile.reputation_score >= TIER_TRUSTED {
                TrustTier::Trusted
            } else if profile.reputation_score >= TIER_ESTABLISHED {
                TrustTier::Established
            } else {
                TrustTier::New
            };

            if new_tier > profile.trust_tier {
                profile.trust_tier = new_tier;
                profile.submissions_per_hour = new_tier.submissions_per_hour();
            }
        }
    }

    /// Check for sybil-like behavior: burst of submissions targeting the same entity
    pub fn check_rate_limit(&mut self, contributor_id: ContributorId) -> bool {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);

        let recent = self.recent_submissions.entry(contributor_id).or_default();

        // Prune old entries
        recent.retain(|t| *t > one_hour_ago);

        let limit = self
            .profiles
            .get(&contributor_id)
            .map_or(5, |p| p.submissions_per_hour);

        if recent.len() >= limit as usize {
            return false; // rate limited
        }

        recent.push(now);
        true
    }

    /// Promote a contributor to a specific tier (admin action)
    pub fn set_tier(&mut self, contributor_id: ContributorId, tier: TrustTier) {
        if let Some(profile) = self.profiles.get_mut(&contributor_id) {
            profile.trust_tier = tier;
            profile.submissions_per_hour = tier.submissions_per_hour();
        }
    }

    /// Unsuspend a contributor (admin action)
    pub fn unsuspend(&mut self, contributor_id: ContributorId) {
        if let Some(profile) = self.profiles.get_mut(&contributor_id) {
            profile.suspended = false;
            profile.suspension_reason = None;
        }
    }

    /// Leaderboard: top N contributors by reputation score
    pub fn leaderboard(&self, n: usize) -> Vec<&ContributorProfile> {
        let mut profiles: Vec<_> = self.profiles.values().filter(|p| !p.suspended).collect();
        profiles.sort_by(|a, b| b.reputation_score.cmp(&a.reputation_score));
        profiles.truncate(n);
        profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("testuser", "abc123hash");
        let profile = tracker.get_profile(id).unwrap();
        assert_eq!(profile.display_name, "testuser");
        assert_eq!(profile.reputation_score, 0);
        assert_eq!(profile.trust_tier, TrustTier::New);
    }

    #[test]
    fn test_approval_increases_score() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("contributor", "hash");
        tracker.record_result(id, SubmissionId::new(), SubmissionStatus::Approved);
        assert_eq!(tracker.get_profile(id).unwrap().reputation_score, 10);
    }

    #[test]
    fn test_tier_promotion() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("power_user", "hash");
        // 5 approvals = 50 points = Established
        for _ in 0..5 {
            tracker.record_result(id, SubmissionId::new(), SubmissionStatus::Approved);
        }
        assert_eq!(
            tracker.get_profile(id).unwrap().trust_tier,
            TrustTier::Established
        );

        // 20 total = 200 points = Trusted
        for _ in 0..15 {
            tracker.record_result(id, SubmissionId::new(), SubmissionStatus::Approved);
        }
        assert_eq!(
            tracker.get_profile(id).unwrap().trust_tier,
            TrustTier::Trusted
        );
    }

    #[test]
    fn test_auto_suspend_bad_actor() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("bad_actor", "hash");
        // 4 rejections = -12 score, 0 approvals, >= 3 rejections → suspended
        for _ in 0..4 {
            tracker.record_result(id, SubmissionId::new(), SubmissionStatus::Rejected);
        }
        let profile = tracker.get_profile(id).unwrap();
        assert!(profile.suspended);
        assert!(profile.suspension_reason.is_some());
    }

    #[test]
    fn test_rate_limiting() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("spammer", "hash");
        // New tier = 5 per hour
        for _ in 0..5 {
            assert!(tracker.check_rate_limit(id));
        }
        // 6th should be blocked
        assert!(!tracker.check_rate_limit(id));
    }

    #[test]
    fn test_leaderboard() {
        let mut tracker = ReputationTracker::new();
        let id1 = tracker.register("alice", "h1");
        let id2 = tracker.register("bob", "h2");
        let _id3 = tracker.register("charlie", "h3");

        for _ in 0..10 {
            tracker.record_result(id1, SubmissionId::new(), SubmissionStatus::Approved);
        }
        for _ in 0..5 {
            tracker.record_result(id2, SubmissionId::new(), SubmissionStatus::Approved);
        }

        let board = tracker.leaderboard(2);
        assert_eq!(board.len(), 2);
        assert_eq!(board[0].display_name, "alice");
        assert_eq!(board[1].display_name, "bob");
    }

    #[test]
    fn test_bad_faith_suspends() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("malicious", "hash");
        tracker.record_bad_faith(
            id,
            "intentionally submitting fabricated connections".to_string(),
        );
        let profile = tracker.get_profile(id).unwrap();
        assert!(profile.suspended);
        assert_eq!(profile.reputation_score, -25);
    }

    #[test]
    fn test_unsuspend() {
        let mut tracker = ReputationTracker::new();
        let id = tracker.register("reformed", "hash");
        tracker.record_bad_faith(id, "test".to_string());
        assert!(tracker.get_profile(id).unwrap().suspended);
        tracker.unsuspend(id);
        assert!(!tracker.get_profile(id).unwrap().suspended);
    }
}
