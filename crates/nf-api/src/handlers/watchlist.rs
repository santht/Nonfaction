use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use nf_store::repository::Repository;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Watchlist types ──────────────────────────────────────────────────────────

/// A watchlist subscription for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub subscribed_at: DateTime<Utc>,
    pub notify_on: Vec<NotifyEvent>,
}

/// Event types that trigger a notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotifyEvent {
    /// Any update to the entity's data.
    EntityUpdate,
    /// A new relationship is added.
    NewRelationship,
    /// A new timing correlation is detected.
    NewCorrelation,
    /// A new source document is linked.
    NewDocument,
}

impl Default for NotifyEvent {
    fn default() -> Self {
        Self::EntityUpdate
    }
}

/// Request body for POST /watchlist/subscribe
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub entity_id: Uuid,
    /// Which events to monitor (defaults to all).
    pub notify_on: Option<Vec<NotifyEvent>>,
}

/// In-memory watchlist store.
/// In production this would be backed by a database table.
#[derive(Default, Clone)]
pub struct WatchlistStore {
    inner: Arc<Mutex<HashMap<Uuid, Subscription>>>,
}

impl WatchlistStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, sub: Subscription) {
        let mut map = self.inner.lock().unwrap();
        map.insert(sub.id, sub);
    }

    pub fn list(&self) -> Vec<Subscription> {
        let map = self.inner.lock().unwrap();
        map.values().cloned().collect()
    }

    pub fn remove(&self, id: Uuid) -> bool {
        let mut map = self.inner.lock().unwrap();
        map.remove(&id).is_some()
    }

    pub fn get(&self, id: Uuid) -> Option<Subscription> {
        let map = self.inner.lock().unwrap();
        map.get(&id).cloned()
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/v1/watchlist/subscribe
///
/// Subscribe to updates for an entity.
pub async fn subscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> ApiResult<(StatusCode, Json<Subscription>)> {
    // Verify entity exists.
    let entity: nf_core::entities::Entity = state
        .entity_repo
        .get(req.entity_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("entity {}", req.entity_id)))?;

    let notify_on = req.notify_on.unwrap_or_else(|| {
        vec![
            NotifyEvent::EntityUpdate,
            NotifyEvent::NewRelationship,
            NotifyEvent::NewCorrelation,
            NotifyEvent::NewDocument,
        ]
    });

    let sub = Subscription {
        id: Uuid::new_v4(),
        entity_id: req.entity_id,
        entity_type: Some(entity.type_name().to_string()),
        entity_name: Some(entity_name_str(&entity)),
        subscribed_at: Utc::now(),
        notify_on,
    };

    state.watchlist.subscribe(sub.clone());

    Ok((StatusCode::CREATED, Json(sub)))
}

/// GET /api/v1/watchlist
///
/// Returns all active subscriptions.
pub async fn list_subscriptions(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Subscription>>> {
    let mut subs = state.watchlist.list();
    // Sort by most recently subscribed first.
    subs.sort_by(|a, b| b.subscribed_at.cmp(&a.subscribed_at));
    Ok(Json(subs))
}

/// DELETE /api/v1/watchlist/:id
///
/// Remove a subscription by its ID.
pub async fn delete_subscription(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if state.watchlist.remove(id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("subscription {id}")))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn entity_name_str(entity: &nf_core::entities::Entity) -> String {
    use nf_core::entities::Entity;
    match entity {
        Entity::Person(p) => p.name.clone(),
        Entity::Organization(o) => o.name.clone(),
        Entity::Document(d) => d.title.clone(),
        Entity::Payment(p) => format!("${:.0} payment", p.amount),
        Entity::CourtCase(c) => c.case_id.clone(),
        Entity::Pardon(p) => format!("Pardon: {}", p.offense),
        Entity::FlightLogEntry(f) => format!("Flight {}", f.aircraft_tail_number),
        Entity::TimingCorrelation(t) => {
            format!("{}→{}", t.event_a_description, t.event_b_description)
        }
        Entity::ConductComparison(c) => c.official_action.clone(),
        Entity::PublicStatement(s) => s.content_summary.clone(),
        Entity::PolicyDecision(p) => p.description.clone(),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sub(entity_id: Uuid) -> Subscription {
        Subscription {
            id: Uuid::new_v4(),
            entity_id,
            entity_type: Some("Person".to_string()),
            entity_name: Some("Jane Doe".to_string()),
            subscribed_at: Utc::now(),
            notify_on: vec![NotifyEvent::EntityUpdate, NotifyEvent::NewRelationship],
        }
    }

    #[test]
    fn test_watchlist_store_subscribe_and_list() {
        let store = WatchlistStore::new();
        assert!(store.list().is_empty());

        let entity_id = Uuid::new_v4();
        let sub = make_sub(entity_id);
        let sub_id = sub.id;
        store.subscribe(sub);

        let subs = store.list();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].entity_id, entity_id);
        assert_eq!(subs[0].id, sub_id);
    }

    #[test]
    fn test_watchlist_store_remove() {
        let store = WatchlistStore::new();
        let sub = make_sub(Uuid::new_v4());
        let sub_id = sub.id;
        store.subscribe(sub);

        assert!(store.remove(sub_id));
        assert!(store.list().is_empty());
        // Second remove returns false.
        assert!(!store.remove(sub_id));
    }

    #[test]
    fn test_watchlist_store_get() {
        let store = WatchlistStore::new();
        let entity_id = Uuid::new_v4();
        let sub = make_sub(entity_id);
        let sub_id = sub.id;
        store.subscribe(sub);

        let found = store.get(sub_id).unwrap();
        assert_eq!(found.entity_id, entity_id);

        let not_found = store.get(Uuid::new_v4());
        assert!(not_found.is_none());
    }

    #[test]
    fn test_notify_event_serialization() {
        let event = NotifyEvent::NewCorrelation;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"new_correlation\"");

        let deserialized: NotifyEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, NotifyEvent::NewCorrelation);
    }

    #[test]
    fn test_subscription_serialization_roundtrip() {
        let sub = make_sub(Uuid::new_v4());
        let json = serde_json::to_string(&sub).unwrap();
        let recovered: Subscription = serde_json::from_str(&json).unwrap();
        assert_eq!(sub.id, recovered.id);
        assert_eq!(sub.entity_id, recovered.entity_id);
        assert_eq!(sub.notify_on.len(), recovered.notify_on.len());
    }
}
