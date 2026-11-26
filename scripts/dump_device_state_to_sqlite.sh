#!/usr/bin/env bash
set -euo pipefail

: "${SOURCE_DATABASE_URL:?Set SOURCE_DATABASE_URL to the live Postgres URL}"

output_path="${1:-rusty_home.sqlite}"

if [[ -e "$output_path" ]]; then
    printf 'Refusing to overwrite existing file: %s\n' "$output_path" >&2
    exit 1
fi

if ! sqlite3 :memory: 'CREATE TABLE strict_check (value INTEGER) STRICT;' >/dev/null 2>&1; then
    printf 'SQLite 3.37 or newer is required for STRICT tables\n' >&2
    exit 1
fi

output_dir=$(dirname "$output_path")
output_name=$(basename "$output_path")
temporary_dir=$(mktemp -d)
staging_path=""
cleanup() {
    rm -rf "$temporary_dir"
    if [[ -n "$staging_path" ]]; then
        rm -f "$staging_path"
    fi
}
trap cleanup EXIT

staging_path=$(mktemp "${output_dir}/.${output_name}.tmp.XXXXXX")
tag_csv="$temporary_dir/device_state_tag.csv"
state_csv="$temporary_dir/device_state.csv"
availability_csv="$temporary_dir/item_availability.csv"
trigger_csv="$temporary_dir/user_trigger.csv"
energy_csv="$temporary_dir/energy_reading.csv"

quote_sql_literal() {
    local escaped=${1//\'/\'\'}
    printf "'%s'" "$escaped"
}

source_url_sql=$(quote_sql_literal "$SOURCE_DATABASE_URL")
tag_csv_sql=$(quote_sql_literal "$tag_csv")
state_csv_sql=$(quote_sql_literal "$state_csv")
availability_csv_sql=$(quote_sql_literal "$availability_csv")
trigger_csv_sql=$(quote_sql_literal "$trigger_csv")
energy_csv_sql=$(quote_sql_literal "$energy_csv")

duckdb -no-agent -bail <<SQL
LOAD postgres;

ATTACH $source_url_sql AS source (TYPE POSTGRES, READ_ONLY);

COPY (
    SELECT
        row_number() OVER (ORDER BY channel, name)::BIGINT AS id,
        channel,
        name
    FROM source.thing_value_tag
    ORDER BY channel, name
) TO $tag_csv_sql (HEADER, DELIMITER ',');

COPY (
    SELECT
        source,
        item,
        epoch_ms(last_seen) AS last_seen_ms,
        CASE WHEN marked_offline THEN 1 ELSE 0 END AS marked_offline,
        epoch_ms(entry_updated) AS entry_updated_ms,
        CASE WHEN disabled THEN 1 ELSE 0 END AS disabled
    FROM source.item_availability
    ORDER BY source, item
) TO $availability_csv_sql (HEADER, DELIMITER ',');

COPY (
    WITH source_tag_map AS (
        SELECT
            id AS source_tag_id,
            row_number() OVER (ORDER BY channel, name)::BIGINT AS tag_id
        FROM source.thing_value_tag
    )
    SELECT
        row_number() OVER (ORDER BY v.timestamp ASC, v.id ASC)::BIGINT AS id,
        m.tag_id,
        v.value,
        epoch_ms(v.timestamp) AS timestamp_ms
    FROM source.thing_value v
    JOIN source_tag_map m ON m.source_tag_id = v.tag_id
    ORDER BY v.timestamp ASC, v.id ASC
) TO $state_csv_sql (HEADER, DELIMITER ',');

COPY (
    SELECT
        id,
        CAST(trigger AS VARCHAR) AS trigger,
        epoch_us(timestamp) AS timestamp_us,
        epoch_us(active_from) AS active_from_us,
        epoch_us(active_until) AS active_until_us,
        COALESCE(to_json(correlation_id), 'null') AS correlation_id
    FROM source.user_trigger
    ORDER BY id
) TO $trigger_csv_sql (HEADER, DELIMITER ',');

COPY (
    SELECT
        id,
        type,
        name,
        CASE WHEN isfinite(value) THEN value ELSE error('Non-finite energy reading: id ' || id) END AS value,
        epoch_us(timestamp) AS timestamp_us,
        CASE WHEN year_end THEN 1 ELSE 0 END AS year_end
    FROM source.energy_reading
    ORDER BY id
) TO $energy_csv_sql (HEADER, DELIMITER ',');

DETACH source;
SQL

sqlite3 -bail "$staging_path" <<SQL
PRAGMA journal_mode=OFF;
PRAGMA synchronous=OFF;
PRAGMA foreign_keys=ON;

CREATE TABLE device_state_tag (
    id INTEGER PRIMARY KEY,
    channel TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (channel, name)
) STRICT;
CREATE TABLE device_state (
    id INTEGER PRIMARY KEY,
    tag_id INTEGER NOT NULL REFERENCES device_state_tag(id),
    value REAL NOT NULL,
    timestamp_ms INTEGER NOT NULL
) STRICT;

.import --csv --skip 1 "$tag_csv" device_state_tag
.import --csv --skip 1 "$state_csv" device_state

CREATE INDEX device_state_tag_timestamp_id_idx
    ON device_state (tag_id, timestamp_ms DESC, id DESC);

CREATE TABLE item_availability (
    source TEXT NOT NULL,
    item TEXT NOT NULL,
    last_seen_ms INTEGER NOT NULL,
    marked_offline INTEGER NOT NULL CHECK (marked_offline IN (0, 1)),
    entry_updated_ms INTEGER NOT NULL,
    disabled INTEGER NOT NULL CHECK (disabled IN (0, 1)),
    PRIMARY KEY (source, item)
) STRICT;

.import --csv --skip 1 "$availability_csv" item_availability

CREATE TABLE user_trigger (
    id INTEGER PRIMARY KEY,
    trigger TEXT NOT NULL,
    timestamp_us INTEGER NOT NULL,
    active_from_us INTEGER,
    active_until_us INTEGER,
    correlation_id TEXT
) STRICT;

-- SQLite's CSV import uses empty text for empty fields, not SQL NULL.
CREATE TEMP TABLE user_trigger_import (id, trigger, timestamp_us, active_from_us, active_until_us, correlation_id);
.import --csv --skip 1 "$trigger_csv" user_trigger_import
INSERT INTO user_trigger
SELECT id, trigger, timestamp_us, NULLIF(active_from_us, ''), NULLIF(active_until_us, ''), json_extract(correlation_id, '$')
FROM user_trigger_import;
DROP TABLE user_trigger_import;

CREATE INDEX user_trigger_timestamp_id_idx
    ON user_trigger (timestamp_us DESC, id DESC);

CREATE TABLE energy_reading (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    timestamp_us INTEGER NOT NULL,
    year_end INTEGER NOT NULL CHECK (year_end IN (0, 1))
) STRICT;

.import --csv --skip 1 "$energy_csv" energy_reading

ANALYZE;
.headers on
.mode column
SELECT 'device_state_tag' AS table_name, count(*) AS rows FROM device_state_tag
UNION ALL
SELECT 'device_state', count(*) FROM device_state
UNION ALL
SELECT 'item_availability', count(*) FROM item_availability
UNION ALL
SELECT 'user_trigger', count(*) FROM user_trigger
UNION ALL
SELECT 'energy_reading', count(*) FROM energy_reading;
SQL

ln "$staging_path" "$output_path"
rm -f "$staging_path"
staging_path=""
printf 'Wrote %s\n' "$output_path"
