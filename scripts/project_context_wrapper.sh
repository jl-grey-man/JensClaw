#!/bin/bash
# project_context_wrapper.sh
# Wrapper that checks project context before allowing work

PROJECT_NAME="$1"
COMMAND="$2"

if [ -z "$PROJECT_NAME" ]; then
    echo "ERROR: Usage: project_context_wrapper.sh <project_name> <command>"
    exit 1
fi

# Convert to lowercase for consistency
PROJECT_LOWER=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]')

# Check if context has been reviewed
CONTEXT_FLAG="/tmp/sandy_context_${PROJECT_LOWER}_reviewed"

if [ ! -f "$CONTEXT_FLAG" ]; then
    echo "⚠️  CONTEXT CHECK REQUIRED FOR PROJECT: $PROJECT_NAME"
    echo ""
    /home/jens/sandy/scripts/check_project_context.sh "$PROJECT_LOWER"
    echo ""
    echo "============================================"
    echo "BLOCKING: Context not reviewed yet"
    echo "============================================"
    echo ""
    echo "To proceed after reviewing files:"
    echo "  touch $CONTEXT_FLAG"
    echo ""
    echo "Then re-run your command."
    exit 1
fi

# Context reviewed, allow work
echo "✓ Context reviewed for $PROJECT_NAME - proceeding with work"
eval "$COMMAND"
