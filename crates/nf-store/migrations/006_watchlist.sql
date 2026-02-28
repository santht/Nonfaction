-- Watchlist subscriptions
CREATE TABLE IF NOT EXISTS watchlist_subscriptions (
    id UUID PRIMARY KEY,
    subscriber_id UUID NOT NULL,
    watch_type TEXT NOT NULL,
    watch_data JSONB NOT NULL,
    notify_preference TEXT NOT NULL DEFAULT 'immediate',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_watchlist_subscriber ON watchlist_subscriptions (subscriber_id);
CREATE INDEX IF NOT EXISTS idx_watchlist_active ON watchlist_subscriptions (active) WHERE active = TRUE;

-- Watchlist alerts
CREATE TABLE IF NOT EXISTS watchlist_alerts (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES watchlist_subscriptions(id),
    subscriber_id UUID NOT NULL,
    alert_type TEXT NOT NULL,
    message TEXT NOT NULL,
    entity_id UUID,
    read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alerts_subscriber_unread ON watchlist_alerts (subscriber_id, read) WHERE read = FALSE;
