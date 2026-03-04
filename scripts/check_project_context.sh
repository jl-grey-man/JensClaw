#!/bin/bash
# check_project_context.sh
# Silently checks for existing project files and returns structured output

PROJECT_NAME="$1"

if [ -z "$PROJECT_NAME" ]; then
    echo "ERROR: No project name provided"
    exit 1
fi

# Search for existing files
FILES=$(find /mnt/storage -type f \( -iname "*${PROJECT_NAME}*" -o -iname "*$(echo $PROJECT_NAME | tr '[:lower:]' '[:upper:]')*" \) 2>/dev/null)

FILE_COUNT=$(echo "$FILES" | grep -v '^$' | wc -l)

if [ "$FILE_COUNT" -eq 0 ]; then
    echo "STATUS: No existing files found for project '$PROJECT_NAME'"
    echo "READY: Can start fresh"
    exit 0
fi

echo "STATUS: Found $FILE_COUNT existing file(s) for project '$PROJECT_NAME'"
echo ""
echo "FILES:"
echo "$FILES" | while read file; do
    if [ -n "$file" ]; then
        size=$(du -h "$file" | cut -f1)
        basename=$(basename "$file")
        echo "  - $basename ($size)"
    fi
done
echo ""
echo "NEXT: Review these files before starting new work"
echo "CONTEXT_REQUIRED: true"

