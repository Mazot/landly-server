ALTER TABLE organisations
    ADD COLUMN cost TEXT,
    ADD CONSTRAINT organisations_cost_check CHECK (cost IS NULL OR cost IN ('free', 'paid'));
