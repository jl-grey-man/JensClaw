#!/bin/bash
# Sandy Watchdog - Simple version that auto-updates and restarts
# Run this on your Pi with: nohup ./watchdog.sh &

SANDY_DIR="/home/jens/sandy"
LOG_FILE="/home/jens/sandy/logs/watchdog.log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log "Sandy Watchdog started"

while true; do
    cd "$SANDY_DIR" || exit 1
    
    # Fetch latest from GitHub
    git fetch origin main
    
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main)
    
    if [ "$LOCAL" != "$REMOTE" ]; then
        log "New version available, updating..."
        
        # Stop Sandy gracefully
        pkill -f "microclaw start" || true
        sleep 2
        
        # Pull and build
        git pull origin main
        cargo build --release
        
        if [ $? -eq 0 ]; then
            log "Build successful, starting Sandy..."
            
            # Start Sandy in background
            mkdir -p logs
            nohup ./target/release/microclaw start >> logs/sandy.log 2>&1 &
            
            log "Sandy restarted with new version"
        else
            log "ERROR: Build failed!"
        fi
    fi
    
    # Check if Sandy is still running
    if ! pgrep -f "microclaw start" > /dev/null; then
        log "Sandy not running, restarting..."
        mkdir -p logs
        nohup ./target/release/microclaw start >> logs/sandy.log 2>&1 &
        log "Sandy restarted"
    fi
    
    # Check every 5 minutes
    sleep 300
done
