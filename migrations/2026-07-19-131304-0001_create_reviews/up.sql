-- Reviews with a polymorphic target: exactly one of organisation_id/person_id
-- (design: org-full.jsx reviews block, person.jsx)
CREATE TABLE reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organisation_id UUID REFERENCES organisations(id) ON DELETE CASCADE,
    person_id UUID REFERENCES people(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL,
    topic TEXT,
    text TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT reviews_rating_check CHECK (rating BETWEEN 1 AND 5),
    CONSTRAINT reviews_single_target CHECK (num_nonnulls(organisation_id, person_id) = 1)
);

-- One review per author per target
CREATE UNIQUE INDEX idx_reviews_author_org ON reviews(author_id, organisation_id) WHERE organisation_id IS NOT NULL;
CREATE UNIQUE INDEX idx_reviews_author_person ON reviews(author_id, person_id) WHERE person_id IS NOT NULL;
CREATE INDEX idx_reviews_org ON reviews(organisation_id) WHERE organisation_id IS NOT NULL;
CREATE INDEX idx_reviews_person ON reviews(person_id) WHERE person_id IS NOT NULL;
