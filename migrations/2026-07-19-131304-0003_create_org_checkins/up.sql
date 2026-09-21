-- Community check-ins: "I was here, still active" + tips
-- (design: org-full.jsx "Community check-ins" block)
CREATE TABLE org_checkins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organisation_id UUID NOT NULL REFERENCES organisations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    still_active BOOLEAN NOT NULL DEFAULT TRUE,
    tip TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_org_checkins_org ON org_checkins(organisation_id);
