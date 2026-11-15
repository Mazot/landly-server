-- Two more ways to reach a recommended helper: Telegram and Instagram handles. Like email and WhatsApp
-- they are hidden until the person confirms, and published only per their own toggle.
ALTER TABLE people
    ADD COLUMN telegram TEXT,
    ADD COLUMN instagram TEXT,
    ADD COLUMN show_telegram BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN show_instagram BOOLEAN NOT NULL DEFAULT FALSE;

-- The claim link can be sent through any of the four contacts.
ALTER TABLE people DROP CONSTRAINT people_send_via_check;
ALTER TABLE people ADD CONSTRAINT people_send_via_check
    CHECK (send_via IS NULL OR send_via IN ('email', 'whatsapp', 'telegram', 'instagram'));
