ALTER TABLE item_availability
    ADD COLUMN disabled BOOLEAN NOT NULL DEFAULT false;
