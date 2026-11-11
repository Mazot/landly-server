-- The organisation `cost` (free / paid) field is gone: nothing used it.
ALTER TABLE organisations
    DROP CONSTRAINT organisations_cost_check,
    DROP COLUMN cost;
