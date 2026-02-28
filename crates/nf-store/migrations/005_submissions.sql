-- Submissions table for crowdsourced data contributions
CREATE TABLE IF NOT EXISTS submissions (
    id UUID PRIMARY KEY,
    contributor_id UUID NOT NULL,
    submission_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    primary_source_url TEXT NOT NULL,
    primary_source_type TEXT NOT NULL,
    reference_detail TEXT NOT NULL,
    description TEXT NOT NULL,
    data JSONB NOT NULL,
    reviewer_id UUID,
    review_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_submissions_status ON submissions (status);
CREATE INDEX IF NOT EXISTS idx_submissions_contributor ON submissions (contributor_id);
CREATE INDEX IF NOT EXISTS idx_submissions_created ON submissions (created_at DESC);

-- Contributor reputation profiles
CREATE TABLE IF NOT EXISTS contributors (
    id UUID PRIMARY KEY,
    display_name TEXT NOT NULL,
    email_hash TEXT NOT NULL,
    reputation_score BIGINT NOT NULL DEFAULT 0,
    total_submissions BIGINT NOT NULL DEFAULT 0,
    approved_submissions BIGINT NOT NULL DEFAULT 0,
    rejected_submissions BIGINT NOT NULL DEFAULT 0,
    trust_tier TEXT NOT NULL DEFAULT 'New',
    suspended BOOLEAN NOT NULL DEFAULT FALSE,
    suspension_reason TEXT,
    submissions_per_hour INTEGER NOT NULL DEFAULT 5,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Review actions audit trail
CREATE TABLE IF NOT EXISTS review_actions (
    id UUID PRIMARY KEY,
    submission_id UUID NOT NULL REFERENCES submissions(id),
    reviewer_id UUID NOT NULL,
    action TEXT NOT NULL,
    rationale TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_review_actions_submission ON review_actions (submission_id);
CREATE INDEX IF NOT EXISTS idx_review_actions_reviewer ON review_actions (reviewer_id);
