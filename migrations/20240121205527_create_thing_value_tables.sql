CREATE TABLE thing_value_tag (
    id SERIAL PRIMARY KEY,
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    UNIQUE NULLS NOT DISTINCT (channel, name)
);

CREATE TABLE thing_value (
    id BIGSERIAL NOT NULL,
    tag_id INTEGER REFERENCES THING_VALUE_TAG(id) NOT NULL,
    value FLOAT8 NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Add index to the main table
CREATE INDEX idx_thing_value_timestamp ON thing_value(timestamp);
CREATE INDEX idx_thing_value_tag_id_timestamp ON thing_value (tag_id, timestamp DESC);
CREATE INDEX idx_thing_value_tag_id_value_timestamp ON thing_value (tag_id, value, timestamp DESC);

-- for example: select create_thing_value_partition('2024-11');
CREATE OR REPLACE FUNCTION create_thing_value_partition(target_year_month TEXT) RETURNS VOID AS $$
DECLARE
    start_date TIMESTAMPTZ;
    end_date TIMESTAMPTZ;
    partition_name TEXT;
BEGIN
    -- Parse the input string to construct the start and end dates
    start_date := to_timestamp(target_year_month || '-01', 'YYYY-MM-DD');
    end_date := start_date + INTERVAL '1 month';
    
    -- Construct the partition name based on the start date
    partition_name := 'thing_value_' || to_char(start_date, 'YYYY_MM');
    
    -- Execute the SQL to create the partition and indexes
    EXECUTE 'CREATE TABLE IF NOT EXISTS ' || partition_name || ' PARTITION OF thing_value FOR VALUES FROM (''' || start_date || ''') TO (''' || end_date || ''');';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_timestamp ON ' || partition_name || '(timestamp);';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_tag_id_timestamp ON ' || partition_name || '(tag_id, timestamp DESC);';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_tag_id_value_timestamp ON ' || partition_name || '(tag_id, value, timestamp DESC);';
END $$ LANGUAGE plpgsql;

SELECT create_thing_value_partition(to_char(current_date, 'YYYY-MM'));


CREATE OR REPLACE FUNCTION notify_thing_value_insert() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'thing_values_insert', 
        json_build_object(
            'id', NEW.id,
            'tag_id', NEW.tag_id
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER thing_value_insert_notify
AFTER INSERT ON thing_value
FOR EACH ROW EXECUTE FUNCTION notify_thing_value_insert();


