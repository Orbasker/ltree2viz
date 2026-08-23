-- Seed data for the ltree2mmd demo: a small product catalog stored as an ltree
-- hierarchy. Runs automatically the first time the postgres container boots.

CREATE EXTENSION IF NOT EXISTS ltree;

CREATE TABLE catalog (
    id   serial PRIMARY KEY,
    path ltree NOT NULL
);

INSERT INTO catalog (path) VALUES
    ('Electronics'),
    ('Electronics.Computers'),
    ('Electronics.Computers.Laptops'),
    ('Electronics.Computers.Desktops'),
    ('Electronics.Phones'),
    ('Electronics.Phones.Android'),
    ('Electronics.Phones.iOS'),
    ('Home'),
    ('Home.Kitchen'),
    ('Home.Kitchen.Cookware'),
    ('Home.Garden'),
    ('Home.Garden.Tools');
