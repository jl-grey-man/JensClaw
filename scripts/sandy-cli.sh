#!/bin/bash
# Sandy CLI Helper for Raspberry Pi
# Usage: Just type 'sandy' in terminal

show_help() {
    cat << 'SANDY_HELP'
╔════════════════════════════════════════════════════════════╗
║                   🧠 SANDY QUICK REFERENCE                 ║
╚════════════════════════════════════════════════════════════╝

📱 ACCESS SANDY:
  Telegram:     @sandy_adhd_coach_bot
  Web UI:       http://100.72.180.20:3000 (via Tailscale)

📊 CHECK STATUS:
  sandy status      - Check if Sandy is running
  sandy logs        - View real-time logs
  sandy update      - Force check for updates

🔧 MANAGE SANDY:
  sandy start       - Start Sandy (if not running)
  sandy stop        - Stop Sandy
  sandy restart     - Restart Sandy

📁 LOCATIONS:
  Sandy Code:     ~/sandy/
  Data Files:     ~/sandy/soul/data/
  User Files:     /mnt/storage/
  Logs:           ~/sandy/logs/

📝 KEY LOG FILES:
  Watchdog Logs:  ~/sandy/logs/watchdog.log
  Sandy Output:   ~/sandy/logs/sandy.log
  Activity:       ~/sandy/soul/data/runtime/activity_log.json

🔄 AUTO-UPDATE:
  Status:         ps aux | grep watchdog
  Last Check:     tail -1 ~/sandy/logs/watchdog.log
  Config:         crontab -l | grep sandy

💡 QUICK TIPS:
  • Sandy checks GitHub every 5 minutes for updates
  • Auto-restarts if she crashes
  • Auto-starts on boot (via crontab)
  • Use Ctrl+C to exit log view

For more help: ~/sandy/QUICK_DEPLOY.md

SANDY_HELP
}

# Command handling
case "${1:-help}" in
    status|s)
        echo "Checking Sandy status..."
        if pgrep -f "microclaw start" > /dev/null; then
            echo "✅ Sandy is running"
            echo ""
            echo "Process info:"
            ps aux | grep -E "(microclaw|watchdog)" | grep -v grep
            echo ""
            echo "Uptime:"
            ps -o pid,etime,command -p $(pgrep -f "microclaw start")
        else
            echo "❌ Sandy is NOT running"
            echo "Start with: sandy start"
        fi
        ;;
    
    logs|log|l)
        echo "📊 Showing Sandy logs (Ctrl+C to exit)..."
        echo ""
        if [ -f ~/sandy/logs/watchdog.log ]; then
            echo "=== Watchdog Logs (auto-update) ==="
            tail -f ~/sandy/logs/watchdog.log 2>/dev/null &
            TAIL_PID=$!
            sleep 0.5
            read -p "Press Enter to stop viewing..."
            kill $TAIL_PID 2>/dev/null
        else
            echo "No logs found yet. Logs will appear after Sandy starts."
        fi
        ;;
    
    start)
        echo "🚀 Starting Sandy..."
        if pgrep -f "microclaw start" > /dev/null; then
            echo "Sandy is already running!"
            sandy status
        else
            cd ~/sandy
            nohup ./target/release/microclaw start > logs/sandy.log 2>&1 &
            sleep 2
            echo "✅ Sandy started"
            sandy status
        fi
        ;;
    
    stop)
        echo "🛑 Stopping Sandy..."
        pkill -f "microclaw start"
        echo "✅ Sandy stopped"
        ;;
    
    restart)
        echo "🔄 Restarting Sandy..."
        sandy stop
        sleep 2
        sandy start
        ;;
    
    update|u)
        echo "🔄 Checking for updates..."
        cd ~/sandy
        git fetch origin main
        LOCAL=$(git rev-parse HEAD)
        REMOTE=$(git rev-parse origin/main)
        
        if [ "$LOCAL" != "$REMOTE" ]; then
            echo "📥 Updates available! Pulling..."
            git pull origin main
            echo "🔨 Rebuilding..."
            cargo build --release
            echo "✅ Update complete! Restarting..."
            sandy restart
        else
            echo "✅ Already up to date"
        fi
        ;;
    
    edit-config|config)
        echo "⚙️  Opening config file..."
        nano ~/sandy/sandy.config.yaml
        ;;
    
    web|w)
        echo "🌐 Sandy Web UI:"
        echo "  Local:     http://localhost:3000"
        echo "  Tailscale: http://100.72.180.20:3000"
        echo ""
        echo "Opening in browser..."
        xdg-open http://localhost:3000 2>/dev/null || \
        sensible-browser http://localhost:3000 2>/dev/null || \
        echo "Please open: http://localhost:3000"
        ;;
    
    storage|files)
        echo "📁 User files location: /mnt/storage/"
        ls -la /mnt/storage/
        ;;
    
    help|--help|-h|*)
        show_help
        ;;
esac
