-- `longitude` was DECIMAL(10, 8): two integer digits, max ±99.99999999. The
-- API accepts -180..180, so every place east of 100°E or west of 100°W
-- (East Asia, the Americas) failed on insert with "numeric field overflow".
-- DECIMAL(11, 8) holds ±180 with the same 8 decimals. Widening never loses
-- data; the (latitude, longitude) index is rebuilt automatically.
ALTER TABLE organisations ALTER COLUMN longitude TYPE DECIMAL(11, 8);
