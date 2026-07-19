-- Reports with a polymorphic target (design: org-detail, person, messaging)
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id UUID REFERENCES users(id) ON DELETE SET NULL,
    target_kind TEXT NOT NULL,
    target_id UUID NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT reports_target_kind_check CHECK (target_kind IN ('org', 'person', 'conversation')),
    CONSTRAINT reports_status_check CHECK (status IN ('open', 'resolved', 'dismissed'))
);

CREATE INDEX idx_reports_target ON reports(target_kind, target_id);
CREATE INDEX idx_reports_status ON reports(status);
