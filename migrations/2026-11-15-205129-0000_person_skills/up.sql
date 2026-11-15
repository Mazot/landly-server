-- What a recommended helper can help with: the choices the "Recommend someone" form and the people
-- filter offer. `people.skills` keeps the chosen names as before; this table is the reference list.
CREATE TABLE skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    sort_order INTEGER NOT NULL UNIQUE
);

INSERT INTO skills (name, sort_order) VALUES
    ('Relocation', 1),
    ('Paperwork', 2),
    ('Housing', 3),
    ('Banking', 4),
    ('Taxes', 5),
    ('Jobs', 6),
    ('Language', 7),
    ('Kids & school', 8),
    ('Medical', 9);

-- The people list can be narrowed to the corridor's destination country.
CREATE INDEX people_location_country_idx ON people (location_country_id);
