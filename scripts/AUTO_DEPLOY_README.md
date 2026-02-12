# Sandy Auto-Deployment Guide for Raspberry Pi

This guide sets up Sandy to:
1. ✅ Automatically pull updates from GitHub
2. ✅ Auto-restart after updates
3. ✅ Auto-start on Pi boot
4. ✅ Stay running 24/7

---

## Method 1: Simple Watchdog (Recommended for now)

The watchdog script runs in the background, checks for updates every 5 minutes, and handles everything.

### Step 1: Copy scripts to your Pi

From your Mac:
```bash
cd ~/Documents/-ai_projects-/SandyNew
scp scripts/watchdog.sh scripts/auto-update.sh pi@192.168.1.46:~/sandy/scripts/
scp .env pi@192.168.1.46:~/sandy/
```

### Step 2: Setup on the Pi

SSH to your Pi:
```bash
ssh pi@192.168.1.46
cd ~/sandy

# Make scripts executable
chmod +x scripts/watchdog.sh scripts/auto-update.sh

# Create log directory
mkdir -p logs

# Start the watchdog
nohup ./scripts/watchdog.sh > logs/watchdog.log 2>&1 &

# Check it's running
ps aux | grep watchdog
tail -f logs/watchdog.log
```

### Step 3: Auto-start on boot (Optional but recommended)

Add to crontab:
```bash
crontab -e
```

Add this line at the end:
```
@reboot cd /home/jens/sandy && nohup ./scripts/watchdog.sh > /home/jens/sandy/logs/watchdog.log 2>&1 &
```

Save and exit. Now Sandy will auto-start on every boot.

---

## Method 2: Systemd Service (More robust)

For a more production-ready setup with proper process management.

### Step 1: Copy service file

From your Mac:
```bash
scp scripts/sandy.service pi@192.168.1.46:/tmp/
```

### Step 2: Install service on Pi

On the Pi:
```bash
ssh pi@192.168.1.46

# Copy service file
sudo cp /tmp/sandy.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Enable service (auto-start on boot)
sudo systemctl enable sandy

# Start the service
sudo systemctl start sandy

# Check status
sudo systemctl status sandy
```

### Step 3: Setup auto-updater (separate from service)

The auto-updater runs alongside the service:
```bash
cd ~/sandy
nohup ./scripts/auto-update.sh > /var/log/sandy-updater.log 2>&1 &
```

Add to crontab for boot:
```bash
crontab -e
```

Add:
```
@reboot cd /home/jens/sandy && nohup ./scripts/auto-update.sh > /var/log/sandy-updater.log 2>&1 &
```

---

## How It Works

### Watchdog Method (Method 1):
1. Runs in background (nohup)
2. Every 5 minutes:
   - Checks GitHub for updates: `git fetch`
   - Compares local vs remote commits
   - If different: stops Sandy, pulls, rebuilds, restarts
   - If Sandy crashed: automatically restarts it
3. Logs everything to `logs/watchdog.log`

### Systemd Method (Method 2):
1. Systemd manages Sandy process
2. Auto-restarts if crashes
3. Separate updater script checks GitHub
4. Updater restarts systemd service when updates found
5. Both start automatically on boot

---

## Monitoring

### Check if Sandy is running:
```bash
# Method 1 (Watchdog)
ps aux | grep watchdog
ps aux | grep microclaw

# Method 2 (Systemd)
sudo systemctl status sandy
```

### View logs:
```bash
# Watchdog logs
tail -f ~/sandy/logs/watchdog.log

# Sandy logs
tail -f ~/sandy/logs/sandy.log

# Systemd logs
sudo journalctl -u sandy -f
```

### Manual control:
```bash
# If using watchdog method:
# Stop watchdog
pkill -f watchdog.sh

# Stop Sandy
pkill -f "microclaw start"

# Start manually
./target/release/microclaw start

# If using systemd method:
sudo systemctl stop sandy
sudo systemctl start sandy
sudo systemctl restart sandy
```

---

## Troubleshooting

### Issue: "Permission denied" on scripts
**Fix:** `chmod +x scripts/*.sh`

### Issue: "TAVILY_API_KEY not set"
**Fix:** Copy .env file to Pi: `scp .env pi@<ip>:~/sandy/`

### Issue: Build fails after update
**Fix:** Check logs, manually run `cargo build --release` to see errors

### Issue: Git pull fails (conflicts)
**Fix:**
```bash
cd ~/sandy
git stash
git pull origin main
git stash pop
cargo build --release
```

---

## Security Notes

1. **API Keys:** Stored in `.env` file (not committed to git)
2. **SSH:** Uses your existing SSH keys for GitHub access
3. **Logs:** May contain sensitive info, check them regularly
4. **Permissions:** Scripts run as user 'jens', not root (safer)

---

## Recommended Setup

**For now, use Method 1 (Watchdog) because it's simpler and easier to debug.**

Once everything is stable, you can switch to Method 2 (Systemd) for better process management.

**Current status:** Scripts created, ready to deploy to Pi.
