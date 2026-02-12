#!/bin/bash
# Sandy Setup Script for Raspberry Pi
# Run this after copying files to /mnt/storage/

echo "🔧 Sandy Auto-Deployment Setup"
echo "================================"

# Check if running as correct user
if [ "$USER" != "jens" ]; then
    echo "⚠️  Warning: You are running as '$USER', expected 'jens'"
    echo "Continue anyway? (y/n)"
    read -r response
    if [ "$response" != "y" ]; then
        exit 1
    fi
fi

SANDY_DIR="$HOME/sandy"
STORAGE_DIR="/mnt/storage"

# Check if sandy directory exists
if [ ! -d "$SANDY_DIR" ]; then
    echo "❌ Error: $SANDY_DIR not found!"
    echo "Please clone/pull the repository first:"
    echo "  git clone https://github.com/jl-grey-man/JensClaw.git ~/sandy"
    exit 1
fi

# Check if files are in storage
if [ ! -f "$STORAGE_DIR/watchdog.sh" ]; then
    echo "❌ Error: watchdog.sh not found in $STORAGE_DIR"
    echo "Please copy it from your Mac first."
    exit 1
fi

if [ ! -f "$STORAGE_DIR/.env" ]; then
    echo "❌ Error: .env file not found in $STORAGE_DIR"
    echo "Please copy it from your Mac first."
    exit 1
fi

echo ""
echo "📁 Moving files to correct locations..."

# Create scripts directory if needed
mkdir -p "$SANDY_DIR/scripts"

# Copy files
cp "$STORAGE_DIR/watchdog.sh" "$SANDY_DIR/scripts/"
cp "$STORAGE_DIR/.env" "$SANDY_DIR/"

# Make executable
chmod +x "$SANDY_DIR/scripts/watchdog.sh"

echo "✅ Files copied"
echo ""

# Create logs directory
mkdir -p "$SANDY_DIR/logs"

echo "🚀 Starting Sandy..."

# Stop any existing Sandy process
pkill -f "microclaw start" 2>/dev/null || true
sleep 2

# Start the watchdog
cd "$SANDY_DIR"
nohup ./scripts/watchdog.sh > logs/watchdog.log 2>&1 &

echo "✅ Watchdog started"
echo ""

# Check if it's running
sleep 2
if pgrep -f "watchdog.sh" > /dev/null; then
    echo "✅ Watchdog is running!"
    echo ""
    echo "📊 Checking status:"
    ps aux | grep -E "(watchdog|microclaw)" | grep -v grep
    echo ""
    echo "📝 Recent log entries:"
    tail -n 5 "$SANDY_DIR/logs/watchdog.log" 2>/dev/null || echo "(Log file not created yet, check in a few seconds)"
else
    echo "❌ Warning: Watchdog may not have started properly"
    echo "Check logs: tail -f $SANDY_DIR/logs/watchdog.log"
fi

echo ""
echo "📋 Setup complete!"
echo ""
echo "Next steps:"
echo "1. Check logs: tail -f $SANDY_DIR/logs/watchdog.log"
echo "2. Test: Make a commit on GitHub, it will auto-deploy in ~5 minutes"
echo "3. Auto-start on boot: Run 'crontab -e' and add:"
echo "   @reboot cd $SANDY_DIR && nohup ./scripts/watchdog.sh > $SANDY_DIR/logs/watchdog.log 2>&1 &"
echo ""
echo "For help: see /mnt/storage/QUICK_DEPLOY.md or /mnt/storage/AUTO_DEPLOY_README.md"
