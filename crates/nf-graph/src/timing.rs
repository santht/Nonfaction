use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use nf_core::entities::{CorrelationType, EntityId, EntityMeta, TimingCorrelation};
use nf_core::source::{ContentHash, SourceChain, SourceRef, SourceType};
use url::Url;

/// The type of event recorded in the timing engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    Donation,
    Vote,
    Indictment,
    Pardon,
    Lobbying,
}

/// A single recorded event for a specific entity.
#[derive(Debug, Clone)]
pub struct TimingEvent {
    pub entity_id: EntityId,
    pub event_type: EventType,
    pub date: NaiveDate,
}

/// A dated money movement between two entities.
#[derive(Debug, Clone, PartialEq)]
pub struct MoneyFlow {
    pub donor: EntityId,
    pub recipient: EntityId,
    pub amount: f64,
    pub date: NaiveDate,
}

/// Aggregate money-flow metrics over a fixed period.
#[derive(Debug, Clone, PartialEq)]
pub struct MoneyFlowWindow {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_amount: f64,
    pub transfer_count: usize,
    pub unique_donors: usize,
    pub unique_recipients: usize,
}

/// A temporally consistent chain of transfers.
#[derive(Debug, Clone, PartialEq)]
pub struct MoneyFlowChain {
    pub path: Vec<EntityId>,
    pub transfer_count: usize,
    pub total_amount: f64,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Timing engine that stores events and auto-generates timing correlations.
///
/// Auto-flag rules:
/// - `Donation → Vote` within 90 days
/// - `Indictment → Pardon` always
/// - `Lobbying → Vote` within 180 days
pub struct TimingEngine {
    events: Vec<TimingEvent>,
}

impl TimingEngine {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Record a new event and return any auto-flagged timing correlations with
    /// previously stored events for the same entity.
    pub fn new_event(
        &mut self,
        entity_id: EntityId,
        event_type: EventType,
        date: NaiveDate,
    ) -> Vec<TimingCorrelation> {
        let new_ev = TimingEvent {
            entity_id,
            event_type,
            date,
        };
        let mut correlations = Vec::new();

        for existing in &self.events {
            if existing.entity_id != entity_id {
                continue;
            }

            // Order events chronologically.
            let (earlier, later) = if existing.date <= new_ev.date {
                (existing, &new_ev)
            } else {
                (&new_ev, existing)
            };

            let days = (later.date - earlier.date).num_days() as u32;

            if let Some((corr_type, threshold)) =
                correlation_rule(&earlier.event_type, &later.event_type)
            {
                let auto_flagged = match threshold {
                    Some(t) => days < t,
                    None => true,
                };

                if auto_flagged {
                    correlations.push(TimingCorrelation {
                        meta: EntityMeta::new(system_source_chain()),
                        event_a: entity_id,
                        event_a_description: format!("{:?}", earlier.event_type),
                        event_a_date: earlier.date,
                        event_b: entity_id,
                        event_b_description: format!("{:?}", later.event_type),
                        event_b_date: later.date,
                        days_between: days,
                        correlation_type: corr_type,
                        auto_flagged: true,
                        threshold_days: threshold,
                    });
                }
            }
        }

        self.events.push(new_ev);
        correlations
    }

    /// All stored events.
    pub fn events(&self) -> &[TimingEvent] {
        &self.events
    }
}

impl Default for TimingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporal network analyzer for money flows.
pub struct TemporalNetworkAnalyzer {
    flows: Vec<MoneyFlow>,
}

impl TemporalNetworkAnalyzer {
    pub fn new() -> Self {
        Self { flows: Vec::new() }
    }

    pub fn add_flow(&mut self, donor: EntityId, recipient: EntityId, amount: f64, date: NaiveDate) {
        self.flows.push(MoneyFlow {
            donor,
            recipient,
            amount,
            date,
        });
    }

    pub fn flows(&self) -> &[MoneyFlow] {
        &self.flows
    }

    /// Return all flows whose date is in [`start`, `end`].
    pub fn flows_in_window(&self, start: NaiveDate, end: NaiveDate) -> Vec<MoneyFlow> {
        self.flows
            .iter()
            .filter(|flow| flow.date >= start && flow.date <= end)
            .cloned()
            .collect()
    }

    /// Aggregate flow volume into fixed-width date buckets.
    pub fn aggregate_by_period(&self, period_days: i64) -> Vec<MoneyFlowWindow> {
        if self.flows.is_empty() || period_days <= 0 {
            return Vec::new();
        }

        let min_date = self.flows.iter().map(|flow| flow.date).min().unwrap();
        let mut buckets: HashMap<i64, Vec<&MoneyFlow>> = HashMap::new();

        for flow in &self.flows {
            let offset_days = (flow.date - min_date).num_days();
            let bucket = offset_days / period_days;
            buckets.entry(bucket).or_default().push(flow);
        }

        let mut keys: Vec<i64> = buckets.keys().copied().collect();
        keys.sort_unstable();

        keys.into_iter()
            .map(|bucket| {
                let entries = buckets.get(&bucket).unwrap();
                let start_date = min_date + Duration::days(bucket * period_days);
                let end_date = start_date + Duration::days(period_days - 1);
                let total_amount = entries.iter().map(|flow| flow.amount).sum();
                let unique_donors: HashSet<EntityId> =
                    entries.iter().map(|flow| flow.donor).collect();
                let unique_recipients: HashSet<EntityId> =
                    entries.iter().map(|flow| flow.recipient).collect();

                MoneyFlowWindow {
                    start_date,
                    end_date,
                    total_amount,
                    transfer_count: entries.len(),
                    unique_donors: unique_donors.len(),
                    unique_recipients: unique_recipients.len(),
                }
            })
            .collect()
    }

    /// Detect transfer chains with at least `min_transfers` hops and no gap
    /// larger than `max_gap_days` between consecutive transfers.
    pub fn detect_flow_chains(
        &self,
        max_gap_days: i64,
        min_transfers: usize,
    ) -> Vec<MoneyFlowChain> {
        if self.flows.is_empty() || max_gap_days < 0 || min_transfers == 0 {
            return Vec::new();
        }

        let mut flows = self.flows.clone();
        flows.sort_by_key(|flow| flow.date);

        let mut adjacency: HashMap<EntityId, Vec<usize>> = HashMap::new();
        for (idx, flow) in flows.iter().enumerate() {
            adjacency.entry(flow.donor).or_default().push(idx);
        }

        let mut results = Vec::new();
        let mut signatures = HashSet::new();

        for start_idx in 0..flows.len() {
            let start = &flows[start_idx];
            let mut path = vec![start.donor, start.recipient];
            let mut indices = vec![start_idx];
            let mut visited = HashSet::new();
            visited.insert(start_idx);

            walk_chains(
                &flows,
                &adjacency,
                start_idx,
                max_gap_days,
                min_transfers,
                &mut path,
                &mut indices,
                &mut visited,
                &mut signatures,
                &mut results,
            );
        }

        results
    }
}

impl Default for TemporalNetworkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

fn walk_chains(
    flows: &[MoneyFlow],
    adjacency: &HashMap<EntityId, Vec<usize>>,
    current_idx: usize,
    max_gap_days: i64,
    min_transfers: usize,
    path: &mut Vec<EntityId>,
    indices: &mut Vec<usize>,
    visited: &mut HashSet<usize>,
    signatures: &mut HashSet<String>,
    results: &mut Vec<MoneyFlowChain>,
) {
    let current = &flows[current_idx];
    let mut extended = false;

    if let Some(next_candidates) = adjacency.get(&current.recipient) {
        for next_idx in next_candidates {
            if visited.contains(next_idx) {
                continue;
            }

            let next = &flows[*next_idx];
            if next.date < current.date {
                continue;
            }
            if (next.date - current.date).num_days() > max_gap_days {
                continue;
            }

            visited.insert(*next_idx);
            path.push(next.recipient);
            indices.push(*next_idx);

            walk_chains(
                flows,
                adjacency,
                *next_idx,
                max_gap_days,
                min_transfers,
                path,
                indices,
                visited,
                signatures,
                results,
            );

            indices.pop();
            path.pop();
            visited.remove(next_idx);
            extended = true;
        }
    }

    if !extended {
        let transfer_count = indices.len();
        if transfer_count >= min_transfers {
            let start_date = indices.iter().map(|idx| flows[*idx].date).min().unwrap();
            let end_date = indices.iter().map(|idx| flows[*idx].date).max().unwrap();
            let total_amount = indices.iter().map(|idx| flows[*idx].amount).sum();

            let signature = path
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<String>>()
                .join("->");

            if signatures.insert(signature) {
                results.push(MoneyFlowChain {
                    path: path.clone(),
                    transfer_count,
                    total_amount,
                    start_date,
                    end_date,
                });
            }
        }
    }
}

/// Return the correlation type and day threshold for a given (earlier, later)
/// event-type pair, or `None` if the pair is not tracked.
fn correlation_rule(
    earlier: &EventType,
    later: &EventType,
) -> Option<(CorrelationType, Option<u32>)> {
    match (earlier, later) {
        (EventType::Donation, EventType::Vote) => Some((CorrelationType::DonationToVote, Some(90))),
        (EventType::Indictment, EventType::Pardon) => {
            Some((CorrelationType::IndictmentToPardon, None))
        }
        (EventType::Lobbying, EventType::Vote) => {
            Some((CorrelationType::LobbyingToVote, Some(180)))
        }
        _ => None,
    }
}

fn system_source_chain() -> SourceChain {
    let url = Url::parse("https://nonfaction.org/system/timing-engine").unwrap();
    let source = SourceRef::new(
        url,
        ContentHash::compute(b"nf-graph-timing-engine"),
        SourceType::OtherGovernment,
        "system",
    );
    SourceChain::new(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_core::entities::CorrelationType;

    #[test]
    fn test_donation_to_vote_within_threshold_flagged() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let mar1 = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();

        engine.new_event(entity, EventType::Donation, jan1);
        let corrs = engine.new_event(entity, EventType::Vote, mar1);

        assert_eq!(corrs.len(), 1);
        let c = &corrs[0];
        assert_eq!(c.correlation_type, CorrelationType::DonationToVote);
        assert!(c.auto_flagged);
        assert_eq!(c.threshold_days, Some(90));
        assert_eq!(c.days_between, 60);
    }

    #[test]
    fn test_donation_to_vote_outside_threshold_not_flagged() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let may1 = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap();

        engine.new_event(entity, EventType::Donation, jan1);
        let corrs = engine.new_event(entity, EventType::Vote, may1);

        assert!(corrs.is_empty());
    }

    #[test]
    fn test_indictment_to_pardon_always_flagged() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan = NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();
        let far = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();

        engine.new_event(entity, EventType::Indictment, jan);
        let corrs = engine.new_event(entity, EventType::Pardon, far);

        assert_eq!(corrs.len(), 1);
        assert_eq!(
            corrs[0].correlation_type,
            CorrelationType::IndictmentToPardon
        );
        assert!(corrs[0].auto_flagged);
        assert_eq!(corrs[0].threshold_days, None);
    }

    #[test]
    fn test_lobbying_to_vote_within_threshold() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let jun1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();

        engine.new_event(entity, EventType::Lobbying, jan1);
        let corrs = engine.new_event(entity, EventType::Vote, jun1);

        assert_eq!(corrs.len(), 1);
        assert_eq!(corrs[0].correlation_type, CorrelationType::LobbyingToVote);
        assert_eq!(corrs[0].threshold_days, Some(180));
    }

    #[test]
    fn test_lobbying_to_vote_outside_threshold_not_flagged() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let aug1 = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();

        engine.new_event(entity, EventType::Lobbying, jan1);
        let corrs = engine.new_event(entity, EventType::Vote, aug1);

        assert!(corrs.is_empty());
    }

    #[test]
    fn test_different_entities_no_correlation() {
        let mut engine = TimingEngine::new();
        let entity_a = EntityId::new();
        let entity_b = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let feb1 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();

        engine.new_event(entity_a, EventType::Donation, jan1);
        let corrs = engine.new_event(entity_b, EventType::Vote, feb1);

        assert!(corrs.is_empty());
    }

    #[test]
    fn test_reverse_order_still_detected() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let mar1 = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();

        engine.new_event(entity, EventType::Vote, mar1);
        let corrs = engine.new_event(entity, EventType::Donation, jan1);

        assert_eq!(corrs.len(), 1);
        assert_eq!(corrs[0].correlation_type, CorrelationType::DonationToVote);
        assert_eq!(corrs[0].days_between, 60);
    }

    #[test]
    fn test_no_correlation_for_untracked_pair() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let feb1 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();

        engine.new_event(entity, EventType::Donation, jan1);
        let corrs = engine.new_event(entity, EventType::Donation, feb1);

        assert!(corrs.is_empty());
    }

    #[test]
    fn test_events_stored() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        engine.new_event(entity, EventType::Donation, date);
        assert_eq!(engine.events().len(), 1);
    }

    #[test]
    fn test_temporal_flows_in_window() {
        let mut analyzer = TemporalNetworkAnalyzer::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        analyzer.add_flow(a, b, 100.0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        analyzer.add_flow(b, c, 150.0, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());

        let flows = analyzer.flows_in_window(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        );
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].amount, 100.0);
    }

    #[test]
    fn test_temporal_aggregate_by_period() {
        let mut analyzer = TemporalNetworkAnalyzer::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        analyzer.add_flow(a, b, 100.0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        analyzer.add_flow(b, c, 200.0, NaiveDate::from_ymd_opt(2024, 1, 10).unwrap());
        analyzer.add_flow(c, a, 300.0, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());

        let windows = analyzer.aggregate_by_period(30);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].transfer_count, 2);
        assert!((windows[0].total_amount - 300.0).abs() < 1e-9);
    }

    #[test]
    fn test_temporal_detect_flow_chain() {
        let mut analyzer = TemporalNetworkAnalyzer::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        let d = EntityId::new();
        analyzer.add_flow(a, b, 50.0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        analyzer.add_flow(b, c, 75.0, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        analyzer.add_flow(c, d, 80.0, NaiveDate::from_ymd_opt(2024, 1, 25).unwrap());

        let chains = analyzer.detect_flow_chains(20, 3);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].path, vec![a, b, c, d]);
        assert_eq!(chains[0].transfer_count, 3);
        assert!((chains[0].total_amount - 205.0).abs() < 1e-9);
    }

    #[test]
    fn test_temporal_detect_flow_chain_gap_breaks_chain() {
        let mut analyzer = TemporalNetworkAnalyzer::new();
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        analyzer.add_flow(a, b, 50.0, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        analyzer.add_flow(b, c, 75.0, NaiveDate::from_ymd_opt(2024, 4, 15).unwrap());

        let chains = analyzer.detect_flow_chains(30, 2);
        assert!(chains.is_empty());
    }
}
