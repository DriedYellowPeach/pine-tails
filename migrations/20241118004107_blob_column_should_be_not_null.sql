-- Add migration script here
DELETE FROM posts
WHERE blob IS NULL;

ALTER TABLE posts
ALTER COLUMN blob SET NOT NULL;
