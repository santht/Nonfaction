use chrono::NaiveDate;
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

            // Order events chronologically
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
                    None => true, // IndictmentToPardon: always flag
                };

                if auto_flagged {
                    let tc = TimingCorrelation {
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
                    };
                    correlations.push(tc);
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

/// Return the correlation type and day threshold for a given (earlier, later)
/// event-type pair, or `None` if the pair is not tracked.
fn correlation_rule(
    earlier: &EventType,
    later: &EventType,
) -> Option<(CorrelationType, Option<u32>)> {
    match (earlier, later) {
        (EventType::Donation, EventType::Vote) => {
            Some((CorrelationType::DonationToVote, Some(90)))
        }
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
        let mar1 = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(); // 60 days later

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
        let may1 = NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(); // 121 days later

        engine.new_event(entity, EventType::Donation, jan1);
        let corrs = engine.new_event(entity, EventType::Vote, may1);

        assert!(corrs.is_empty());
    }

    #[test]
    fn test_indictment_to_pardon_always_flagged() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan = NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();
        let far = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(); // years later

        engine.new_event(entity, EventType::Indictment, jan);
        let corrs = engine.new_event(entity, EventType::Pardon, far);

        assert_eq!(corrs.len(), 1);
        assert_eq!(corrs[0].correlation_type, CorrelationType::IndictmentToPardon);
        assert!(corrs[0].auto_flagged);
        assert_eq!(corrs[0].threshold_days, None);
    }

    #[test]
    fn test_lobbying_to_vote_within_threshold() {
        let mut engine = TimingEngine::new();
        let entity = EntityId::new();

        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let jun1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(); // ~152 days

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
        let aug1 = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap(); // ~213 days

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

        // Vote recorded first, then donation that happened earlier
        let jan1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let mar1 = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();

        engine.new_event(entity, EventType::Vote, mar1);
        // Donation happened before the vote (60 days)
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
}
