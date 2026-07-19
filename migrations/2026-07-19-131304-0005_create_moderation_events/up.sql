-- Moderation audit trail; `flags` holds automatic submit-time checks
-- (duplicate nearby, phone format, trusted volunteer) (design: moderation.jsx)
CREATE TABLE moderation_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_kind TEXT NOT NULL,
    target_id UUID NOT NULL,
    moderator_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    note TEXT,
    flags JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT moderation_events_target_kind_check CHECK (target_kind IN ('org', 'person')),
    CONSTRAINT moderation_events_action_check CHECK (action IN ('submitted', 'approve', 'request_changes', 'reject'))
);

CREATE INDEX idx_moderation_events_target ON moderation_events(target_kind, target_id);
