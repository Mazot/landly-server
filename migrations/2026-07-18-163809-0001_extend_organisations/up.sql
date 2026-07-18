-- Organisation v2 fields (design: shared.jsx V2_ORGS, org-full.jsx, map-filters.jsx)
ALTER TABLE organisations
    ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN verified BOOLEAN NOT NULL DEFAULT FALSE,
    -- Existing rows are backfilled as 'live'; new submissions get 'pending' in code.
    ADD COLUMN status TEXT NOT NULL DEFAULT 'live',
    ADD COLUMN moderation_note TEXT,
    ADD COLUMN added_by TEXT,
    ADD COLUMN city TEXT,
    ADD COLUMN website TEXT,
    ADD COLUMN telegram TEXT,
    ADD COLUMN whatsapp TEXT,
    ADD COLUMN services TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN languages TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN opening_hours JSONB,
    ADD COLUMN timezone TEXT,
    ADD COLUMN cost TEXT,
    ADD COLUMN google_place_id TEXT,
    ADD COLUMN google_rating DOUBLE PRECISION,
    ADD COLUMN visits_count BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN rating_avg DOUBLE PRECISION,
    ADD COLUMN reviews_count BIGINT NOT NULL DEFAULT 0;

ALTER TABLE organisations
    ADD CONSTRAINT organisations_status_check CHECK (status IN ('pending', 'live', 'rejected')),
    ADD CONSTRAINT organisations_added_by_check CHECK (added_by IS NULL OR added_by IN ('official', 'community', 'volunteer')),
    ADD CONSTRAINT organisations_cost_check CHECK (cost IS NULL OR cost IN ('free', 'paid'));

CREATE INDEX idx_organisations_lat_lng ON organisations(latitude, longitude);
CREATE INDEX idx_organisations_status ON organisations(status);

-- Canonical org types from the mockups: stable slug + seed
ALTER TABLE organisation_types ADD COLUMN slug TEXT;
CREATE UNIQUE INDEX idx_organisation_types_slug ON organisation_types(slug) WHERE slug IS NOT NULL;

INSERT INTO organisation_types (type, color, title, slug)
SELECT v.type, v.color, v.title, v.slug
FROM (VALUES
    ('embassy',   '#4A6D8C', 'Embassy',   'embassy'),
    ('business',  '#B8814A', 'Business',  'business'),
    ('helper',    '#4F7F5C', 'Helper',    'helper'),
    ('community', '#8B5A8C', 'Community', 'community'),
    ('volunteer', '#C7613F', 'Volunteer', 'volunteer')
) AS v(type, color, title, slug)
WHERE NOT EXISTS (SELECT 1 FROM organisation_types t WHERE t.slug = v.slug);

-- Best-effort backfill of slugs for pre-existing rows with matching type names
UPDATE organisation_types SET slug = lower(type)
WHERE slug IS NULL AND lower(type) IN ('embassy', 'business', 'helper', 'community', 'volunteer')
  AND NOT EXISTS (SELECT 1 FROM organisation_types t2 WHERE t2.slug = lower(organisation_types.type));
