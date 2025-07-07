-- This file should undo anything in `up.sql`
ALTER TABLE organisations 
DROP COLUMN latitude, 
DROP COLUMN longitude;
