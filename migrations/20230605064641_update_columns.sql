-- Add migration script here
ALTER TABLE token ADD COLUMN activated BOOLEAN NOT NULL;
