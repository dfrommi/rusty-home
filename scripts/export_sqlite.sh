#!/usr/bin/env bash
set -Eeuo pipefail

# Export a Rusty Home SQLite database to Parquet and CSV.
#
# Usage:
#   ~/sqlite_backup/export_sqlite.sh ./rusty_home.sqlite
#
# The SQLite database path is required. The export destinations are always
# relative to ~/sqlite_backup.

if [[ $# -ne 1 ]]; then
    printf 'Usage: %s PATH_TO_SQLITE_DATABASE\n' "$0" >&2
    exit 2
fi

SQLITE_DB="$1"
if [[ ! -f "${SQLITE_DB}" ]]; then
    printf 'SQLite database does not exist: %s\n' "${SQLITE_DB}" >&2
    exit 1
fi

BACKUP_DIR="${HOME}/sqlite_backup"
DEVICE_STATE_DIR="${BACKUP_DIR}/device_state"
ENERGY_READING_CSV="${BACKUP_DIR}/energy_reading.csv"

# Use an absolute path so DuckDB resolves the source independently of its
# working directory.
SQLITE_DB="$(cd "$(dirname "${SQLITE_DB}")" && pwd)/$(basename "${SQLITE_DB}")"

mkdir -p "${BACKUP_DIR}"

# Build each export in a temporary sibling. Existing exports remain intact if
# DuckDB or the database connection fails. The temporary paths are on the same
# filesystem, so the final moves are atomic.
device_state_tmp_root="$(mktemp -d "${BACKUP_DIR}/.device_state.tmp.XXXXXX")"
energy_tmp_root="$(mktemp -d "${BACKUP_DIR}/.energy_reading.tmp.XXXXXX")"
device_state_tmp="${device_state_tmp_root}/data"
energy_reading_tmp="${energy_tmp_root}/energy_reading.csv"

cleanup() {
    rm -rf "${device_state_tmp_root}" "${energy_tmp_root}"
}
trap cleanup EXIT

duckdb -no-agent <<SQL
INSTALL sqlite;
LOAD sqlite;

-- SQLite stores UTC Unix timestamps as integer microseconds.
SET TimeZone = 'UTC';
ATTACH '${SQLITE_DB}' AS src (TYPE sqlite);

COPY (
    SELECT
        id,
        channel,
        name,
        value,
        timestamp,
        year(timestamp) AS year,
        month(timestamp) AS month
    FROM (
        SELECT
            d.id,
            t.channel,
            t.name,
            d.value,
            make_timestamptz(d.timestamp_us) AS timestamp
        FROM src.device_state AS d
        JOIN src.device_state_tag AS t
          ON t.id = d.tag_id
    ) AS device_state
    ORDER BY timestamp, id
) TO '${device_state_tmp}' (
    FORMAT parquet,
    COMPRESSION zstd,
    PARTITION_BY (year, month),
    WRITE_PARTITION_COLUMNS true
);

COPY (
    SELECT
        id,
        type,
        name,
        value,
        strftime(timestamp, '%Y-%m-%dT%H:%M:%S.%fZ') AS timestamp,
        year_end
    FROM (
        SELECT
            id,
            type,
            name,
            value,
            make_timestamptz(timestamp_us) AS timestamp,
            year_end
        FROM src.energy_reading
    ) AS energy_reading
    ORDER BY timestamp, id
) TO '${energy_reading_tmp}' (
    FORMAT csv,
    HEADER true,
    DELIMITER ','
);

DETACH src;
SQL

rm -rf "${DEVICE_STATE_DIR}"
mv "${device_state_tmp}" "${DEVICE_STATE_DIR}"
mv -f "${energy_reading_tmp}" "${ENERGY_READING_CSV}"

printf 'Export complete:\n'
printf '  SQLite:  %s\n' "${SQLITE_DB}"
printf '  Parquet: %s\n' "${DEVICE_STATE_DIR}"
printf '  CSV:     %s\n' "${ENERGY_READING_CSV}"
