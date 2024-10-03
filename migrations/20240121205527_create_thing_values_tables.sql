CREATE TABLE TAGS (
    id SERIAL PRIMARY KEY,
    channel TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE NULLS NOT DISTINCT (channel, name)
);

CREATE TABLE THING_VALUES (
    id BIGSERIAL NOT NULL,
    tag_id INTEGER REFERENCES TAGS(id) NOT NULL,
    value FLOAT8 NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Add index to the main table
CREATE INDEX idx_thing_values_timestamp ON THING_VALUES(timestamp);
CREATE INDEX idx_thing_values_tag_id_timestamp ON THING_VALUES (tag_id, timestamp DESC);
CREATE INDEX idx_thing_values_tag_id_value_timestamp ON THING_VALUES (tag_id, value, timestamp DESC);

-- SELECT cron.schedule('weekly-partition-thing-values', '0 4 * * 3', $$ SELECT create_next_month_thing_values_partition(); $$);

CREATE OR REPLACE FUNCTION create_partition(target_year_month TEXT) RETURNS VOID AS $$
DECLARE
    start_date TIMESTAMPTZ;
    end_date TIMESTAMPTZ;
    partition_name TEXT;
BEGIN
    -- Parse the input string to construct the start and end dates
    start_date := to_timestamp(target_year_month || '-01', 'YYYY-MM-DD');
    end_date := start_date + INTERVAL '1 month';
    
    -- Construct the partition name based on the start date
    partition_name := 'thing_values_' || to_char(start_date, 'YYYY_MM');
    
    -- Execute the SQL to create the partition and indexes
    EXECUTE 'CREATE TABLE IF NOT EXISTS ' || partition_name || ' PARTITION OF thing_values FOR VALUES FROM (''' || start_date || ''') TO (''' || end_date || ''');';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_timestamp ON ' || partition_name || '(timestamp);';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_tag_id_timestamp ON ' || partition_name || '(tag_id, timestamp DESC);';
    EXECUTE 'CREATE INDEX IF NOT EXISTS idx_' || partition_name || '_tag_id_value_timestamp ON ' || partition_name || '(tag_id, value, timestamp DESC);';
END $$ LANGUAGE plpgsql;

SELECT create_partition(to_char(current_date, 'YYYY-MM'));

