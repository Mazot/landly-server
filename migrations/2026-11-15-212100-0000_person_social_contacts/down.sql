ALTER TABLE people DROP CONSTRAINT people_send_via_check;
-- Links that went out through the new channels fall back to a channel the old schema knows.
UPDATE people SET send_via = NULL WHERE send_via IN ('telegram', 'instagram');
ALTER TABLE people ADD CONSTRAINT people_send_via_check CHECK (send_via IS NULL OR send_via IN ('email', 'whatsapp'));
ALTER TABLE people
    DROP COLUMN show_instagram,
    DROP COLUMN show_telegram,
    DROP COLUMN instagram,
    DROP COLUMN telegram;
