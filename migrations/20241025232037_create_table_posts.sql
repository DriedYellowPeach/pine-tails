-- Add migration script here
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    date timestamptz NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    content TEXT
);
