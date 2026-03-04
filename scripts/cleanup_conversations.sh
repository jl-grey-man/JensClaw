#!/bin/bash
set -e

# Sandy conversation cleanup script
# Runs nightly via cron: 0 2 * * *
# Archives old conversation .md files, deletes stale archives, checkpoints WAL.
# Does NOT trim exec_log/activity_log or VACUUM while Sandy is running
# (inode swap causes silent data loss; VACUUM can hang on DB locks).

RUNTIME_DIR="/home/jens/sandy/soul/data/runtime"
GROUPS_DIR="$RUNTIME_DIR/groups"
DB_FILE="$RUNTIME_DIR/microclaw.db"
LOG_FILE="/home/jens/sandy/logs/cleanup.log"
DAYS_TO_KEEP=7
ARCHIVE_MAX_DAYS=30

# Ensure log directory exists (it's a symlink to SSD)
mkdir -p "$(dirname "$LOG_FILE")"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S'): $1" >> "$LOG_FILE"
}

log "Cleanup started"

# Archive and purge conversations for ALL groups
if [ -d "$GROUPS_DIR" ]; then
    for conv_dir in "$GROUPS_DIR"/*/conversations; do
        [ -d "$conv_dir" ] || continue

        archive_dir="$conv_dir/archive"
        mkdir -p "$archive_dir"

        # Move conversations older than DAYS_TO_KEEP to archive
        moved=$(find "$conv_dir" -maxdepth 1 -name "*.md" -mtime +$DAYS_TO_KEEP -exec mv {} "$archive_dir/" \; -print | wc -l)

        # Delete archived conversations older than ARCHIVE_MAX_DAYS
        deleted=$(find "$archive_dir" -name "*.md" -mtime +$ARCHIVE_MAX_DAYS -delete -print | wc -l)

        group_id=$(basename "$(dirname "$conv_dir")")
        [ "$moved" -gt 0 ] || [ "$deleted" -gt 0 ] && \
            log "Group $group_id: archived $moved, purged $deleted"
    done
else
    log "WARN: Groups directory not found: $GROUPS_DIR"
fi

# WAL checkpoint (redundant with DB startup, but catches long-running sessions)
if [ -f "$DB_FILE" ] && command -v sqlite3 >/dev/null 2>&1; then
    if sqlite3 "$DB_FILE" "PRAGMA wal_checkpoint(TRUNCATE);" 2>/dev/null; then
        log "WAL checkpoint OK"
    else
        log "WARN: WAL checkpoint failed (Sandy may hold a lock)"
    fi
fi

log "Cleanup finished"
