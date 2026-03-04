#!/bin/bash
# Sandy conversation cleanup script
# Runs nightly to archive old conversation files

CONV_DIR="/home/jens/sandy/soul/data/runtime/groups/8296186575/conversations"
ARCHIVE_DIR="$CONV_DIR/archive"
DAYS_TO_KEEP=7

# Create archive directory if it doesn't exist
mkdir -p "$ARCHIVE_DIR"

# Move conversations older than DAYS_TO_KEEP to archive
find "$CONV_DIR" -maxdepth 1 -name "*.md" -mtime +$DAYS_TO_KEEP -exec mv {} "$ARCHIVE_DIR/" \;

# Optional: Delete archived conversations older than 30 days to save space
find "$ARCHIVE_DIR" -name "*.md" -mtime +30 -delete

# Log cleanup
echo "$(date): Cleaned conversations older than $DAYS_TO_KEEP days" >> /home/jens/sandy/logs/cleanup.log
