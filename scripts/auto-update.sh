#!/bin/bash
# Sandy Auto-Update Script
# Checks for GitHub updates every 5 minutes, pulls if available, and restarts service

SANDY_DIR="/home/jens/sandy"
LOG_FILE="/var/log/sandy-updater.log"
PID_FILE="/tmp/sandy-updater.pid"

# Prevent multiple instances
if [ -f "$PID_FILE" ]; then
    if ps -p $(cat "$PID_FILE") > /dev/null 2>&1; then
        echo "Updater already running"
        exit 0
    fi
fi
echo $$ > "$PID_FILE"

# Cleanup on exit
trap "rm -f $PID_FILE" EXIT

log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log_message "Sandy Updater started"

while true; do
    cd "$SANDY_DIR" || exit 1
    
    # Check for updates
    git fetch origin main 2>&1 | tee -a "$LOG_FILE"
    
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main)
    
    if [ "$LOCAL" != "$REMOTE" ]; then
        log_message "Update found! Local: $LOCAL, Remote: $REMOTE"
        log_message "Pulling updates..."
        
        # Pull updates
        git pull origin main 2>&1 | tee -a "$LOG_FILE"
        
        if [ $? -eq 0 ]; then
            log_message "Update successful, rebuilding..."
            
            # Rebuild
            cargo build --release 2>&1 | tee -a "$LOG_FILE"
            
            if [ $? -eq 0 ]; then
                log_message "Build successful, restarting Sandy..."
                
                # Restart systemd service
                sudo systemctl restart sandy
                
                log_message "Sandy restarted with updates"
            else
                log_message "ERROR: Build failed!"
            fi
        else
            log_message "ERROR: Git pull failed!"
        fi
    fi
    
    # Check every 5 minutes
    sleep 300
done
