# Sandy Web UI

## Overview

Sandy now has a **built-in web dashboard** that you can access from any device on your network (or the internet if you set up port forwarding).

## Features

### 1. Activity Log Feed 📋
- **Real-time tracking** of everything Sandy does
- Shows when goals, projects, tasks, and patterns are created/updated
- Timestamps showing "Just now", "5m ago", "2h ago", etc.
- Keeps last 1000 activities

### 2. Dropdown Lists 📁
Minimalistic dropdown sections for:
- **🎯 Goals** - All your goals with progress percentages
- **📚 Projects** - Projects with their linked goals
- **✅ Tasks** - Tasks organized by status (To Do, In Progress, Done)
- **🧩 Patterns** - ADHD patterns sorted by confidence level

### 3. Stats Overview 📊
Quick stats at the top showing:
- Active Goals
- Tasks To Do
- Tasks In Progress
- Tasks Completed

## Accessing the Web UI

### Local Access (same computer)
```
http://localhost:3000
```

### Network Access (other devices on same WiFi)
```
http://<your-computer-ip>:3000
```

To find your computer's IP:
```bash
# On Mac
ifconfig | grep "inet " | grep -v 127.0.0.1

# On Linux
ip addr show
```

### Internet Access (from anywhere)
You have several options:

#### Option 1: Port Forwarding (Router)
1. Access your router admin panel
2. Forward port 3000 to your computer's IP
3. Access via: `http://<your-public-ip>:3000`

#### Option 2: ngrok (Easiest)
```bash
# Install ngrok
brew install ngrok

# Start tunnel
ngrok http 3000

# Use the HTTPS URL provided (e.g., https://abc123.ngrok.io)
```

#### Option 3: Cloudflare Tunnel
```bash
# Install cloudflared
brew install cloudflared

# Create tunnel
cloudflared tunnel --url http://localhost:3000
```

## How It Works

### Activity Logging
Every action is automatically logged:
- Creating a goal → Logged
- Adding a task → Logged  
- Marking something complete → Logged
- Adding a pattern observation → Logged
- Creating a new pattern → Logged

### Auto-Refresh
The dashboard refreshes every 30 seconds to show new data.
You can also click the "Refresh" button to update immediately.

### Data Storage
- Activity log: `soul/data/activity_log.json`
- Patterns: `soul/data/patterns.json`
- Tracking: `soul/data/tracking.json`

## Configuration

Change the web UI port in `microclaw.config.yaml`:
```yaml
web_port: 3000  # Change to any port you prefer
```

## Security Notes

⚠️ **Important:** The web UI is currently open (no authentication). 

**For local use only:**
- Only use on trusted networks
- Don't expose to the internet without adding authentication

**To add basic security:**
1. Use a reverse proxy (nginx, Caddy)
2. Add basic auth
3. Or use a VPN to access your home network

## API Endpoints

The web UI also provides JSON API endpoints:

```
GET /api/dashboard     - Full dashboard data
GET /api/activity      - Activity log (last 100 entries)
GET /api/goals         - All goals
GET /api/projects      - All projects
GET /api/tasks         - All tasks
```

Example:
```bash
curl http://localhost:3000/api/dashboard | jq
```

## Mobile Friendly

The dashboard is **responsive** and works great on:
- Desktop browsers
- Tablets
- Phones

Just open the URL in any modern web browser!

## Troubleshooting

**Can't access from another device?**
1. Make sure Sandy is running
2. Check firewall: `sudo ufw allow 3000` (Linux)
3. Verify you're on the same network
4. Try using your computer's IP instead of localhost

**Port already in use?**
Change the port in `microclaw.config.yaml`:
```yaml
web_port: 8080  # Try a different port
```

**Dashboard not loading?**
1. Check if Sandy is running: `ps aux | grep microclaw`
2. Check logs for errors
3. Try refreshing the page

## Example Workflow

1. **Talk to Sandy on Telegram**
   - "I need to finish my website by Friday"
   - Sandy creates a goal + tasks

2. **Check progress on Web UI**
   - Open `http://localhost:3000` on your phone
   - See the new goal in the Goals dropdown
   - See activity in the log

3. **Complete a task**
   - Tell Sandy "I finished the homepage"
   - Web UI updates automatically (or click Refresh)
   - See the activity logged

4. **Track patterns**
   - Sandy learns you work better in mornings
   - Pattern confidence increases
   - Visible in the Patterns section

## Future Enhancements

Possible additions:
- [ ] Authentication/login
- [ ] Edit items directly in web UI
- [ ] Calendar view for deadlines
- [ ] Charts/graphs for progress
- [ ] Dark mode
- [ ] Push notifications

---

**Enjoy your Sandy dashboard!** 🧠✨
