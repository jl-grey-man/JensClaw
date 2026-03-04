#!/bin/bash
# Sandy Updater - Single-run update check for systemd timer
# Checks for GitHub updates, pulls if available, rebuilds, and restarts

SANDY_DIR="/home/jens/sandy"
export PATH="$HOME/.cargo/bin:$PATH"

cd "$SANDY_DIR" || exit 1

echo "Checking for updates..."
git fetch origin main 2>&1

# Check if origin/main has commits not in local HEAD
NEW_COMMITS=$(git rev-list HEAD..origin/main --count)

if [ "$NEW_COMMITS" = "0" ]; then
    echo "Already up to date."
    exit 0
fi

echo "Update found! $NEW_COMMITS new commit(s) on origin/main"
echo "Pulling updates..."

git pull origin main 2>&1
if [ $? -ne 0 ]; then
    echo "ERROR: Git pull failed!"
    exit 1
fi

echo "Building..."
cargo build --release 2>&1
if [ $? -ne 0 ]; then
    echo "ERROR: Build failed!"
    exit 1
fi

echo "Build successful, restarting Sandy..."
sudo systemctl restart sandy
echo "Sandy restarted with updates"
