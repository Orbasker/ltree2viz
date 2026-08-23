-- Seed data for the ltree2viz demo.
--
-- Runs automatically the first time the demo Postgres container starts
-- (Postgres executes every file in /docker-entrypoint-initdb.d on init).

CREATE EXTENSION IF NOT EXISTS ltree;

CREATE TABLE catalog (
    id   serial PRIMARY KEY,
    path ltree NOT NULL,
    name text  NOT NULL
);

CREATE INDEX catalog_path_gist ON catalog USING gist (path);

INSERT INTO catalog (path, name) VALUES
    ('Top',                                'Top'),
    ('Top.Science',                        'Science'),
    ('Top.Science.Astronomy',              'Astronomy'),
    ('Top.Science.Astronomy.Astrophysics', 'Astrophysics'),
    ('Top.Science.Astronomy.Cosmology',    'Cosmology'),
    ('Top.Hobbies',                        'Hobbies'),
    ('Top.Hobbies.Amateurs_Astronomy',     'Amateur Astronomy'),
    ('Top.Collections',                    'Collections'),
    ('Top.Collections.Pictures',           'Pictures'),
    ('Top.Collections.Pictures.Astronomy', 'Astronomy');
