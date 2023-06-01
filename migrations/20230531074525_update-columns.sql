-- Add migration script here
ALTER TABLE account ADD COLUMN "role" VARCHAR NOT NULL;

ALTER TABLE share ADD COLUMN description VARCHAR;

ALTER TABLE "table" ADD COLUMN description VARCHAR;
