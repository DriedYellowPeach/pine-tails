-- Add migration script here
CREATE TABLE users(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    username VARCHAR(64) NOT NULL UNIQUE,
    email VARCHAR(128) NOT NULL UNIQUE,
    password VARCHAR(256) NOT NULL
);
