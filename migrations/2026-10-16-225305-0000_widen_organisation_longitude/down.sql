-- Fails with "numeric field overflow" once any row has |longitude| >= 100 —
-- such rows only exist because of the up migration. Find them with
--   SELECT id, name, longitude FROM organisations WHERE abs(longitude) >= 100;
ALTER TABLE organisations ALTER COLUMN longitude TYPE DECIMAL(10, 8);
