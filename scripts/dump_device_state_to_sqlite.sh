#!/usr/bin/env bash
set -euo pipefail

: "${SOURCE_DATABASE_URL:?Set SOURCE_DATABASE_URL to the live Postgres URL}"

output_path="${1:-device_state.sqlite}"

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

quote_sql_literal() {
    local escaped=${1//\'/\'\'}
    printf "'%s'" "$escaped"
}

source_url_sql=$(quote_sql_literal "$SOURCE_DATABASE_URL")
tag_csv_sql=$(quote_sql_literal "$tag_csv")
state_csv_sql=$(quote_sql_literal "$state_csv")

duckdb -no-agent <<SQL
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
ANALYZE;
.headers on
.mode column
SELECT 'device_state_tag' AS table_name, count(*) AS rows FROM device_state_tag
UNION ALL
SELECT 'device_state', count(*) FROM device_state;
SQL

ln "$staging_path" "$output_path"
rm -f "$staging_path"
staging_path=""
printf 'Wrote %s\n' "$output_path"
