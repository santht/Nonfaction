use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nf_core::entities::EntityId;
use nf_core::relationships::RelationshipType;

use crate::submission::ContributorId;

/// Watchlist subscription — journalists subscribe to entities or relationship types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub subscriber: ContributorId,
    pub watch_type: WatchType,
    pub created_at: DateTime<Utc>,
    pub active: bool,
    /// Notification preferences
    pub notify: NotificationPreference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchType {
    /// Watch a specific entity for any changes
    Entity(EntityId),
    /// Watch for any new relationships of a type involving an entity
    EntityRelationship {
        entity_id: EntityId,
        rel_type: RelationshipType,
    },
    /// Watch for any new timing correlations involving an entity
    TimingCorrelations(EntityId),
    /// Watch all entities matching a keyword
    Keyword(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPreference {
    /// Immediate notification on every update
    Immediate,
    /// Daily digest of all updates
    DailyDigest,
    /// Weekly digest
    WeeklyDigest,
}

/// An alert generated when a watched entity/relationship changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub subscriber: ContributorId,
    pub alert_type: AlertType,
    pub message: String,
    pub entity_id: Option<EntityId>,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertType {
    NewRelationship,
    EntityUpdated,
    NewTimingCorrelation,
    NewDocument,
    NewSubmissionApproved,
}

/// Watchlist manager
#[derive(Debug)]
pub struct WatchlistManager {
    subscriptions: HashMap<Uuid, Subscription>,
    /// Index: entity_id → subscription IDs watching that entity
    entity_watchers: HashMap<EntityId, Vec<Uuid>>,
    /// Pending alerts not yet delivered
    pending_alerts: Vec<Alert>,
}

impl WatchlistManager {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            entity_watchers: HashMap::new(),
            pending_alerts: Vec::new(),
        }
    }

    /// Subscribe to watch something
    pub fn subscribe(
        &mut self,
        subscriber: ContributorId,
        watch_type: WatchType,
        notify: NotificationPreference,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let sub = Subscription {
            id,
            subscriber,
            watch_type: watch_type.clone(),
            created_at: Utc::now(),
            active: true,
            notify,
        };

        // Index by entity for fast lookup
        match &watch_type {
            WatchType::Entity(eid) => {
                self.entity_watchers.entry(*eid).or_default().push(id);
            }
            WatchType::EntityRelationship { entity_id, .. } => {
                self.entity_watchers.entry(*entity_id).or_default().push(id);
            }
            WatchType::TimingCorrelations(eid) => {
                self.entity_watchers.entry(*eid).or_default().push(id);
            }
            WatchType::Keyword(_) => {
                // Keywords need full scan, not indexed by entity
            }
        }

        self.subscriptions.insert(id, sub);
        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, subscription_id: Uuid) -> bool {
        if let Some(sub) = self.subscriptions.get_mut(&subscription_id) {
            sub.active = false;
            true
        } else {
            false
        }
    }

    /// List active subscriptions for a contributor
    pub fn list_subscriptions(&self, subscriber: ContributorId) -> Vec<&Subscription> {
        self.subscriptions
            .values()
            .filter(|s| s.subscriber == subscriber && s.active)
            .collect()
    }

    /// Notify all watchers of an entity change
    pub fn notify_entity_change(
        &mut self,
        entity_id: EntityId,
        alert_type: AlertType,
        message: String,
    ) {
        let watcher_ids: Vec<Uuid> = self
            .entity_watchers
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        for sub_id in watcher_ids {
            if let Some(sub) = self.subscriptions.get(&sub_id) {
                if !sub.active {
                    continue;
                }

                // Check if this alert type matches the subscription type
                let matches = match (&sub.watch_type, &alert_type) {
                    (WatchType::Entity(_), _) => true, // watch all changes
                    (WatchType::EntityRelationship { .. }, AlertType::NewRelationship) => true,
                    (WatchType::TimingCorrelations(_), AlertType::NewTimingCorrelation) => true,
                    _ => false,
                };

                if matches {
                    self.pending_alerts.push(Alert {
                        id: Uuid::new_v4(),
                        subscription_id: sub_id,
                        subscriber: sub.subscriber,
                        alert_type: alert_type.clone(),
                        message: message.clone(),
                        entity_id: Some(entity_id),
                        created_at: Utc::now(),
                        read: false,
                    });
                }
            }
        }
    }

    /// Get unread alerts for a subscriber
    pub fn unread_alerts(&self, subscriber: ContributorId) -> Vec<&Alert> {
        self.pending_alerts
            .iter()
            .filter(|a| a.subscriber == subscriber && !a.read)
            .collect()
    }

    /// Mark an alert as read
    pub fn mark_read(&mut self, alert_id: Uuid) {
        if let Some(alert) = self.pending_alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.read = true;
        }
    }

    /// Get alerts needing immediate delivery
    pub fn pending_immediate_alerts(&self) -> Vec<&Alert> {
        self.pending_alerts
            .iter()
            .filter(|a| {
                !a.read
                    && self
                        .subscriptions
                        .get(&a.subscription_id)
                        .is_some_and(|s| s.notify == NotificationPreference::Immediate)
            })
            .collect()
    }

    /// Drain alerts for digest (daily/weekly)
    pub fn digest_alerts(
        &self,
        subscriber: ContributorId,
        pref: NotificationPreference,
    ) -> Vec<&Alert> {
        self.pending_alerts
            .iter()
            .filter(|a| {
                a.subscriber == subscriber
                    && !a.read
                    && self
                        .subscriptions
                        .get(&a.subscription_id)
                        .is_some_and(|s| s.notify == pref)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_and_list() {
        let mut mgr = WatchlistManager::new();
        let user = ContributorId::new();
        let entity = EntityId::new();

        let sub_id = mgr.subscribe(
            user,
            WatchType::Entity(entity),
            NotificationPreference::Immediate,
        );

        let subs = mgr.list_subscriptions(user);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].id, sub_id);
    }

    #[test]
    fn test_entity_change_triggers_alert() {
        let mut mgr = WatchlistManager::new();
        let user = ContributorId::new();
        let entity = EntityId::new();

        mgr.subscribe(
            user,
            WatchType::Entity(entity),
            NotificationPreference::Immediate,
        );

        mgr.notify_entity_change(
            entity,
            AlertType::NewRelationship,
            "New donation connection found".to_string(),
        );

        let alerts = mgr.unread_alerts(user);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::NewRelationship);
    }

    #[test]
    fn test_relationship_watch_filters_alerts() {
        let mut mgr = WatchlistManager::new();
        let user = ContributorId::new();
        let entity = EntityId::new();

        mgr.subscribe(
            user,
            WatchType::EntityRelationship {
                entity_id: entity,
                rel_type: RelationshipType::DonatedTo,
            },
            NotificationPreference::DailyDigest,
        );

        // This should NOT trigger (wrong alert type for relationship watch)
        mgr.notify_entity_change(entity, AlertType::EntityUpdated, "bio updated".to_string());
        assert_eq!(mgr.unread_alerts(user).len(), 0);

        // This SHOULD trigger
        mgr.notify_entity_change(
            entity,
            AlertType::NewRelationship,
            "new donation".to_string(),
        );
        assert_eq!(mgr.unread_alerts(user).len(), 1);
    }

    #[test]
    fn test_unsubscribe_stops_alerts() {
        let mut mgr = WatchlistManager::new();
        let user = ContributorId::new();
        let entity = EntityId::new();

        let sub_id = mgr.subscribe(
            user,
            WatchType::Entity(entity),
            NotificationPreference::Immediate,
        );

        mgr.unsubscribe(sub_id);

        mgr.notify_entity_change(entity, AlertType::NewDocument, "new document".to_string());

        assert_eq!(mgr.unread_alerts(user).len(), 0);
    }

    #[test]
    fn test_mark_read() {
        let mut mgr = WatchlistManager::new();
        let user = ContributorId::new();
        let entity = EntityId::new();

        mgr.subscribe(
            user,
            WatchType::Entity(entity),
            NotificationPreference::Immediate,
        );

        mgr.notify_entity_change(entity, AlertType::EntityUpdated, "update".to_string());

        let alert_id = mgr.unread_alerts(user)[0].id;
        mgr.mark_read(alert_id);
        assert_eq!(mgr.unread_alerts(user).len(), 0);
    }

    #[test]
    fn test_immediate_vs_digest() {
        let mut mgr = WatchlistManager::new();
        let user1 = ContributorId::new();
        let user2 = ContributorId::new();
        let entity = EntityId::new();

        mgr.subscribe(
            user1,
            WatchType::Entity(entity),
            NotificationPreference::Immediate,
        );
        mgr.subscribe(
            user2,
            WatchType::Entity(entity),
            NotificationPreference::DailyDigest,
        );

        mgr.notify_entity_change(entity, AlertType::NewRelationship, "new link".to_string());

        assert_eq!(mgr.pending_immediate_alerts().len(), 1);
        assert_eq!(
            mgr.digest_alerts(user2, NotificationPreference::DailyDigest)
                .len(),
            1
        );
    }
}
