#!/usr/bin/env bash
set -euo pipefail

: "${SOURCE_DATABASE_URL:?Set SOURCE_DATABASE_URL to the live Postgres URL}"

output_path="${1:-device_state.duckdb}"

if [[ -e "$output_path" ]]; then
    printf 'Refusing to overwrite existing file: %s\n' "$output_path" >&2
    exit 1
fi

duckdb -no-agent "$output_path" <<SQL
LOAD postgres;

ATTACH '${SOURCE_DATABASE_URL}' AS source (TYPE postgres, READ_ONLY);

CREATE SEQUENCE device_state_id_seq;
CREATE SEQUENCE device_state_tag_id_seq;

CREATE TABLE device_state_tag (
    id BIGINT PRIMARY KEY DEFAULT nextval('device_state_tag_id_seq'),
    channel VARCHAR NOT NULL,
    name VARCHAR NOT NULL
);

CREATE TABLE device_state (
    id BIGINT NOT NULL DEFAULT nextval('device_state_id_seq'),
    tag_id BIGINT NOT NULL,
    value DOUBLE NOT NULL,
    timestamp_ms BIGINT NOT NULL
);

CREATE TEMP TABLE source_tag_map AS
SELECT
    id AS source_tag_id,
    row_number() OVER (ORDER BY channel, name)::BIGINT AS tag_id,
    channel,
    name
FROM source.thing_value_tag;

INSERT INTO device_state_tag (id, channel, name)
SELECT tag_id, channel, name
FROM source_tag_map
ORDER BY tag_id;

SELECT CASE
    WHEN count(*) > 0 THEN setval('device_state_tag_id_seq', max(id))
END
FROM device_state_tag;

INSERT INTO device_state (tag_id, value, timestamp_ms)
SELECT m.tag_id, v.value, epoch_ms(v.timestamp)
FROM source.thing_value v
JOIN source_tag_map m ON m.source_tag_id = v.tag_id
ORDER BY v.timestamp ASC, v.id ASC;

SELECT count(*) AS device_state_tags FROM device_state_tag;
SELECT count(*) AS device_state_points FROM device_state;

DETACH source;
SQL

printf 'Wrote %s\n' "$output_path"
