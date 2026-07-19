-- Person / Helper: a recommended human (design: add-helper.jsx, person.jsx, claim.jsx)
-- Status flow: pending (submitted, in moderation) -> awaiting (approved,
-- claim link sent) -> confirmed (person agreed) / claimed (linked an account)
-- or declined.
CREATE TABLE people (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    bio TEXT,
    city TEXT,
    location_country_id UUID REFERENCES countries(id) ON DELETE SET NULL,
    skills TEXT[] NOT NULL DEFAULT '{}',
    -- Hidden contacts: never serialized until confirmed + privacy toggles
    email TEXT,
    whatsapp TEXT,
    send_via TEXT,
    consent_given BOOLEAN NOT NULL DEFAULT FALSE,
    status TEXT NOT NULL DEFAULT 'pending',
    show_whatsapp BOOLEAN NOT NULL DEFAULT FALSE,
    show_email BOOLEAN NOT NULL DEFAULT FALSE,
    show_city BOOLEAN NOT NULL DEFAULT TRUE,
    allow_reviews BOOLEAN NOT NULL DEFAULT TRUE,
    recommended_by UUID REFERENCES users(id) ON DELETE SET NULL,
    claimed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    moderation_note TEXT,
    rating_avg DOUBLE PRECISION,
    reviews_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT people_status_check CHECK (status IN ('pending', 'awaiting', 'confirmed', 'claimed', 'declined')),
    CONSTRAINT people_send_via_check CHECK (send_via IS NULL OR send_via IN ('email', 'whatsapp'))
);

CREATE INDEX idx_people_status ON people(status);
CREATE INDEX idx_people_recommended_by ON people(recommended_by);

CREATE TABLE people_to_languages (
    person_id UUID NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    language_id UUID NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    PRIMARY KEY (person_id, language_id)
);

-- Claim token IS the credential for the account-less confirm/decline flow.
CREATE TABLE person_claim_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id UUID NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMP NOT NULL,
    used_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_person_claim_tokens_person ON person_claim_tokens(person_id);

CREATE TABLE person_vouches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id UUID NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT person_vouches_unique UNIQUE (person_id, user_id)
);
