# Sandy Auto-Deployment - Quick Start

## ✅ What You Get

Sandy will now:
1. **Auto-update**: Check GitHub every 5 min, pull & rebuild if updates found
2. **Auto-restart**: Restart after updates or if crashes
3. **Auto-start on boot**: Runs automatically when Pi boots

---

## 🚀 Deploy to Pi (One-Time Setup)

### Step 1: Copy files to Pi
```bash
# From your Mac
cd ~/Documents/-ai_projects-/SandyNew
scp scripts/watchdog.sh pi@192.168.1.46:~/sandy/scripts/
scp .env pi@192.168.1.46:~/sandy/
```

### Step 2: Setup on Pi
```bash
# SSH to Pi
ssh pi@192.168.1.46

# Make executable and start
cd ~/sandy
chmod +x scripts/watchdog.sh
mkdir -p logs

# Start watchdog
nohup ./scripts/watchdog.sh > logs/watchdog.log 2>&1 &

# Verify it's running
ps aux | grep watchdog
tail -f logs/watchdog.log
```

### Step 3: Auto-start on boot
```bash
crontab -e
```

Add this line:
```
@reboot cd /home/jens/sandy && nohup ./scripts/watchdog.sh > /home/jens/sandy/logs/watchdog.log 2>&1 &
```

Save and exit. Done! ✨

---

## 📋 How It Works

The **watchdog script** runs in background and every 5 minutes:
1. Checks GitHub: `git fetch origin main`
2. If updates found: stops Sandy → pulls → rebuilds → restarts
3. If Sandy crashed: automatically restarts it
4. Logs everything to `logs/watchdog.log`

---

## 🔍 Monitor Sandy

**Check if running:**
```bash
ps aux | grep microclaw
```

**View logs:**
```bash
tail -f ~/sandy/logs/watchdog.log
```

**Manual restart:**
```bash
pkill -f "microclaw start"  # Stop
./target/release/microclaw start  # Start
```

---

## 📖 Full Documentation

See `scripts/AUTO_DEPLOY_README.md` for:
- Alternative systemd service method
- Troubleshooting guide
- Security notes
- Monitoring commands

---

## 🎯 Next Steps

1. **Copy scripts to Pi** (Step 1 above)
2. **Start watchdog** (Step 2 above)  
3. **Add to crontab** (Step 3 above)
4. **Test**: Make a commit on GitHub, watch it auto-deploy in ~5 min

---

**Status:** Scripts pushed to GitHub. Ready to deploy!
