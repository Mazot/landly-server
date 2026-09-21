-- Saved / bookmarks: polymorphic kind + target_id without FK (4 kinds);
-- cleanup happens in the owning delete usecases (design: saved.jsx)
CREATE TABLE saved_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    target_id UUID NOT NULL,
    note TEXT,
    list_name TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT saved_items_kind_check CHECK (kind IN ('org', 'person', 'country', 'corridor')),
    CONSTRAINT saved_items_unique UNIQUE (user_id, kind, target_id)
);

CREATE INDEX idx_saved_items_user ON saved_items(user_id);
