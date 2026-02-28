// Temporal pattern analysis for detecting coordination and suspicious timing.
//
// Three pattern types:
//   Burst      — 3+ events within a 7-day window (coordination signal)
//   Periodic   — events clustering at ~90-day intervals (quarterly filing)
//   ResponseChain — entity B acts within N days of entity A (quid pro quo)

use chrono::{DateTime, Duration, Utc};

use nf_core::entities::EntityId;

/// Classification of a detected temporal pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagType {
    /// Three or more events within a 7-day window.
    Burst,
    /// Events clustering at approximately 90-day intervals.
    Periodic,
    /// Entity B acts within N days of entity A.
    ResponseChain,
}

/// A suspicious temporal pattern detected by [`TemporalAnalyzer`].
#[derive(Debug, Clone)]
pub struct TemporalFlag {
    pub flag_type: FlagType,
    /// The events that triggered this flag, as (timestamp, description) pairs.
    pub events: Vec<(DateTime<Utc>, String)>,
    pub description: String,
}

/// Analyzes time-stamped events associated with entities to surface
/// suspicious coordination patterns.
pub struct TemporalAnalyzer {
    events: Vec<(EntityId, DateTime<Utc>, String)>,
}

impl TemporalAnalyzer {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Register a single event for an entity.
    pub fn add_event(
        &mut self,
        entity: EntityId,
        timestamp: DateTime<Utc>,
        description: impl Into<String>,
    ) {
        self.events.push((entity, timestamp, description.into()));
    }

    /// Retrieve all events for `entity`, sorted chronologically.
    fn sorted_events(&self, entity: EntityId) -> Vec<(DateTime<Utc>, String)> {
        let mut ev: Vec<(DateTime<Utc>, String)> = self
            .events
            .iter()
            .filter(|(e, _, _)| *e == entity)
            .map(|(_, t, d)| (*t, d.clone()))
            .collect();
        ev.sort_by_key(|(t, _)| *t);
        ev
    }

    /// Detect **burst** patterns: 3+ events in any rolling 7-day window.
    ///
    /// Returns one flag per distinct burst window found.
    pub fn detect_bursts(&self, entity: EntityId) -> Vec<TemporalFlag> {
        let sorted = self.sorted_events(entity);
        if sorted.len() < 3 {
            return Vec::new();
        }

        let window = Duration::days(7);
        let mut flags = Vec::new();
        let mut i = 0;

        while i < sorted.len() {
            let mut burst: Vec<(DateTime<Utc>, String)> = vec![sorted[i].clone()];
            for j in (i + 1)..sorted.len() {
                if sorted[j].0 - sorted[i].0 <= window {
                    burst.push(sorted[j].clone());
                } else {
                    break;
                }
            }
            if burst.len() >= 3 {
                let count = burst.len();
                flags.push(TemporalFlag {
                    flag_type: FlagType::Burst,
                    events: burst.clone(),
                    description: format!(
                        "Burst: {} events within 7-day window starting {}",
                        count,
                        sorted[i].0.format("%Y-%m-%d")
                    ),
                });
                // Advance past this burst to find the next distinct one.
                i += count;
            } else {
                i += 1;
            }
        }

        flags
    }

    /// Detect **periodic** patterns: events consistently spaced ~90 days apart
    /// (quarterly campaign-finance filing cycles).
    ///
    /// Requires at least 3 events with consecutive intervals within ±14 days of 90.
    pub fn detect_periodic(&self, entity: EntityId) -> Vec<TemporalFlag> {
        let sorted = self.sorted_events(entity);
        if sorted.len() < 3 {
            return Vec::new();
        }

        let target = Duration::days(90);
        let tolerance = Duration::days(14);

        let mut periodic_events = vec![sorted[0].clone()];
        let mut matching_intervals = 0usize;

        for i in 1..sorted.len() {
            let interval = sorted[i].0 - sorted[i - 1].0;
            let deviation = interval - target;
            // Check |deviation| <= tolerance (chrono Duration supports Ord).
            if deviation >= -tolerance && deviation <= tolerance {
                matching_intervals += 1;
                periodic_events.push(sorted[i].clone());
            } else {
                // Reset if the chain is broken, but keep the current event
                // as a potential new chain start.
                if matching_intervals >= 2 {
                    // Already have a valid run — record it before resetting.
                    break;
                }
                periodic_events = vec![sorted[i].clone()];
                matching_intervals = 0;
            }
        }

        if matching_intervals >= 2 {
            vec![TemporalFlag {
                flag_type: FlagType::Periodic,
                events: periodic_events,
                description: format!(
                    "Periodic: {} consecutive ~90-day intervals detected",
                    matching_intervals
                ),
            }]
        } else {
            Vec::new()
        }
    }

    /// Detect **response-chain** patterns: entity B acts within `max_days` of
    /// every event by entity A (e.g., a vote follows a donation).
    pub fn detect_response_chains(
        &self,
        entity_a: EntityId,
        entity_b: EntityId,
        max_days: i64,
    ) -> Vec<TemporalFlag> {
        let a_events = self.sorted_events(entity_a);
        let b_events = self.sorted_events(entity_b);

        if a_events.is_empty() || b_events.is_empty() {
            return Vec::new();
        }

        let window = Duration::days(max_days);
        let mut flags = Vec::new();

        for (a_time, a_desc) in &a_events {
            for (b_time, b_desc) in &b_events {
                let delta = *b_time - *a_time;
                if delta >= Duration::zero() && delta <= window {
                    flags.push(TemporalFlag {
                        flag_type: FlagType::ResponseChain,
                        events: vec![(*a_time, a_desc.clone()), (*b_time, b_desc.clone())],
                        description: format!(
                            "Response chain: entity_b responded {} day(s) after entity_a",
                            delta.num_days()
                        ),
                    });
                }
            }
        }

        flags
    }

    /// Run all detectors (burst + periodic) for a single entity.
    pub fn analyze_all(&self, entity: EntityId) -> Vec<TemporalFlag> {
        let mut flags = self.detect_bursts(entity);
        flags.extend(self.detect_periodic(entity));
        flags
    }
}

impl Default for TemporalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    // ─── Burst tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_burst_detects_three_events_in_seven_days() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        a.add_event(e, dt(2024, 1, 1), "donation 1");
        a.add_event(e, dt(2024, 1, 3), "donation 2");
        a.add_event(e, dt(2024, 1, 6), "donation 3");

        let flags = a.detect_bursts(e);
        assert!(!flags.is_empty(), "Should detect burst");
        assert_eq!(flags[0].flag_type, FlagType::Burst);
        assert_eq!(flags[0].events.len(), 3);
    }

    #[test]
    fn test_burst_no_flag_for_two_events() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        a.add_event(e, dt(2024, 1, 1), "event 1");
        a.add_event(e, dt(2024, 1, 3), "event 2");

        assert!(a.detect_bursts(e).is_empty());
    }

    #[test]
    fn test_burst_no_flag_spread_events() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        a.add_event(e, dt(2024, 1, 1), "e1");
        a.add_event(e, dt(2024, 1, 10), "e2");
        a.add_event(e, dt(2024, 1, 20), "e3");

        assert!(a.detect_bursts(e).is_empty(), "10-day gaps are not a burst");
    }

    #[test]
    fn test_burst_flags_five_events_in_window() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        for day in 1u32..=5 {
            a.add_event(e, dt(2024, 3, day), format!("event {day}"));
        }
        let flags = a.detect_bursts(e);
        assert!(!flags.is_empty());
        assert!(flags[0].events.len() >= 5);
    }

    #[test]
    fn test_burst_empty_entity() {
        let a = TemporalAnalyzer::new();
        assert!(a.detect_bursts(EntityId::new()).is_empty());
    }

    // ─── Periodic tests ───────────────────────────────────────────────────────

    #[test]
    fn test_periodic_detects_quarterly_pattern() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        // Q1, Q2, Q3, Q4 filings ~90 days apart
        a.add_event(e, dt(2024, 1, 15), "Q1 filing");
        a.add_event(e, dt(2024, 4, 14), "Q2 filing"); // 90 days later
        a.add_event(e, dt(2024, 7, 13), "Q3 filing"); // 90 days later
        a.add_event(e, dt(2024, 10, 11), "Q4 filing"); // 90 days later

        let flags = a.detect_periodic(e);
        assert!(!flags.is_empty(), "Should detect quarterly pattern");
        assert_eq!(flags[0].flag_type, FlagType::Periodic);
    }

    #[test]
    fn test_periodic_no_flag_for_random_intervals() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        a.add_event(e, dt(2024, 1, 1), "e1");
        a.add_event(e, dt(2024, 2, 15), "e2"); // 45 days
        a.add_event(e, dt(2024, 6, 1), "e3"); // 107 days
        a.add_event(e, dt(2024, 8, 20), "e4"); // 80 days

        assert!(a.detect_periodic(e).is_empty());
    }

    #[test]
    fn test_periodic_within_tolerance() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        // Intervals of 85, 90, 96 — all within ±14 days of 90
        a.add_event(e, dt(2024, 1, 1), "e1");
        a.add_event(e, dt(2024, 3, 27), "e2"); // 85 days
        a.add_event(e, dt(2024, 6, 25), "e3"); // 90 days
        a.add_event(e, dt(2024, 9, 29), "e4"); // 96 days

        let flags = a.detect_periodic(e);
        assert!(!flags.is_empty(), "All intervals within tolerance should flag");
    }

    #[test]
    fn test_periodic_needs_at_least_three_events() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();
        a.add_event(e, dt(2024, 1, 1), "e1");
        a.add_event(e, dt(2024, 4, 1), "e2"); // 91 days — just 1 interval

        assert!(a.detect_periodic(e).is_empty());
    }

    // ─── Response-chain tests ─────────────────────────────────────────────────

    #[test]
    fn test_response_chain_detected() {
        let mut a = TemporalAnalyzer::new();
        let donor = EntityId::new();
        let politician = EntityId::new();

        a.add_event(donor, dt(2024, 1, 1), "donation $50k");
        a.add_event(politician, dt(2024, 1, 25), "voted yes on bill");

        let flags = a.detect_response_chains(donor, politician, 30);
        assert!(!flags.is_empty());
        assert_eq!(flags[0].flag_type, FlagType::ResponseChain);
        assert_eq!(flags[0].events.len(), 2);
    }

    #[test]
    fn test_response_chain_outside_window() {
        let mut a = TemporalAnalyzer::new();
        let donor = EntityId::new();
        let politician = EntityId::new();

        a.add_event(donor, dt(2024, 1, 1), "donation");
        a.add_event(politician, dt(2024, 6, 1), "vote"); // 152 days later

        let flags = a.detect_response_chains(donor, politician, 30);
        assert!(flags.is_empty(), "152 days is outside 30-day window");
    }

    #[test]
    fn test_response_chain_b_before_a_not_flagged() {
        let mut a = TemporalAnalyzer::new();
        let donor = EntityId::new();
        let politician = EntityId::new();

        a.add_event(donor, dt(2024, 3, 1), "donation");
        a.add_event(politician, dt(2024, 1, 1), "vote"); // before donation

        let flags = a.detect_response_chains(donor, politician, 90);
        assert!(flags.is_empty(), "B acting before A should not be flagged");
    }

    #[test]
    fn test_response_chain_multiple_pairs() {
        let mut a = TemporalAnalyzer::new();
        let donor = EntityId::new();
        let politician = EntityId::new();

        a.add_event(donor, dt(2024, 1, 1), "donation 1");
        a.add_event(donor, dt(2024, 4, 1), "donation 2");
        a.add_event(politician, dt(2024, 1, 20), "vote 1");
        a.add_event(politician, dt(2024, 4, 15), "vote 2");

        let flags = a.detect_response_chains(donor, politician, 30);
        assert!(flags.len() >= 2, "Should detect at least 2 response pairs");
    }

    #[test]
    fn test_response_chain_empty_parties() {
        let a = TemporalAnalyzer::new();
        let flags = a.detect_response_chains(EntityId::new(), EntityId::new(), 30);
        assert!(flags.is_empty());
    }

    // ─── analyze_all tests ────────────────────────────────────────────────────

    #[test]
    fn test_analyze_all_returns_combined_flags() {
        let mut a = TemporalAnalyzer::new();
        let e = EntityId::new();

        // Burst cluster
        a.add_event(e, dt(2024, 1, 1), "ev1");
        a.add_event(e, dt(2024, 1, 3), "ev2");
        a.add_event(e, dt(2024, 1, 5), "ev3");

        // Periodic cluster
        a.add_event(e, dt(2024, 3, 1), "q1");
        a.add_event(e, dt(2024, 5, 30), "q2"); // 90 days
        a.add_event(e, dt(2024, 8, 28), "q3"); // 90 days

        let flags = a.analyze_all(e);
        let has_burst = flags.iter().any(|f| f.flag_type == FlagType::Burst);
        let has_periodic = flags.iter().any(|f| f.flag_type == FlagType::Periodic);
        assert!(has_burst, "analyze_all should include burst flags");
        assert!(has_periodic, "analyze_all should include periodic flags");
    }

    #[test]
    fn test_analyze_all_empty_returns_no_flags() {
        let a = TemporalAnalyzer::new();
        assert!(a.analyze_all(EntityId::new()).is_empty());
    }

    // ─── 20-node scenario tests ───────────────────────────────────────────────

    /// Simulates 20 entities with complex temporal patterns:
    /// - 5 entities each show a burst pattern in their activity windows
    /// - 5 entity pairs show response-chain coordination
    /// - 5 entities show quarterly periodic filing patterns
    #[test]
    fn test_large_scale_temporal_scenario() {
        let mut analyzer = TemporalAnalyzer::new();
        let entities: Vec<EntityId> = (0..20).map(|_| EntityId::new()).collect();

        // Entities 0-4: burst patterns
        for i in 0..5 {
            let e = entities[i];
            for d in 0u32..5 {
                analyzer.add_event(e, dt(2024, 1 + i as u32, 1 + d), format!("burst ev {d}"));
            }
        }

        // Entities 5-9 (donors) + 10-14 (politicians): response chains
        for i in 0..5 {
            let donor = entities[5 + i];
            let pol = entities[10 + i];
            analyzer.add_event(donor, dt(2024, 1, 1), format!("donation from {i}"));
            analyzer.add_event(pol, dt(2024, 1, 15), format!("vote by {i}"));
        }

        // Entities 15-19: quarterly periodic patterns
        for i in 0..5 {
            let e = entities[15 + i];
            for q in 0u32..4 {
                analyzer.add_event(
                    e,
                    dt(2024, 1 + q * 3, 1),
                    format!("quarterly filing {q}"),
                );
            }
        }

        // Verify bursts for first 5 entities
        for i in 0..5 {
            let flags = analyzer.detect_bursts(entities[i]);
            assert!(
                !flags.is_empty(),
                "Entity {i} should have a burst flag"
            );
        }

        // Verify response chains for pairs 5-9 / 10-14
        for i in 0..5 {
            let flags = analyzer.detect_response_chains(entities[5 + i], entities[10 + i], 30);
            assert!(
                !flags.is_empty(),
                "Pair {i} should show response chain"
            );
        }

        // Verify periodic patterns for entities 15-19
        for i in 0..5 {
            let flags = analyzer.detect_periodic(entities[15 + i]);
            assert!(
                !flags.is_empty(),
                "Entity {} should show periodic pattern",
                15 + i
            );
        }
    }
}
