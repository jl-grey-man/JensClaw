---
name: documents
description: Manage documents and files in /mnt/storage. Use this skill when users want to create, read, update, list, or organize files including notes, reports, code (HTML, CSS, JS, Python), lists, and any text-based documents. Supports Markdown, Text, HTML, CSS, JavaScript, JSON, and Python files with subfolder organization.
license: MIT
compatibility:
  os:
    - linux
    - darwin
---

# Documents Skill - File Management Guide

Use this skill when the user wants to:
- Create new documents (notes, reports, lists, code)
- Read existing files
- List directory contents
- Update or append to existing files
- Organize files into subdirectories
- Manage any text-based files

## Supported File Types

- **Markdown (.md)** - Notes, documentation, reports
- **Plain Text (.txt)** - Simple notes, lists
- **HTML (.html)** - Web pages, templates
- **CSS (.css)** - Stylesheets
- **JavaScript (.js)** - Scripts, web apps
- **JSON (.json)** - Data files, configurations
- **Python (.py)** - Scripts, automation

## Core Operations

### 1. Creating Files

**Basic file creation:**
```bash
# Create a note
echo "# Meeting Notes

- Discuss project timeline
- Review budget" > /mnt/storage/notes/meeting-2024-01-15.md

# Create a text list
echo "Shopping List:
- Milk
- Eggs
- Bread" > /mnt/storage/lists/groceries.txt

# Create HTML file
cat > /mnt/storage/projects/website/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>My Page</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>Hello World</h1>
    <script src="app.js"></script>
</body>
</html>
EOF

# Create CSS file
cat > /mnt/storage/projects/website/style.css << 'EOF'
body {
    font-family: Arial, sans-serif;
    margin: 20px;
    background-color: #f5f5f5;
}

h1 {
    color: #333;
}
EOF

# Create JavaScript file
cat > /mnt/storage/projects/website/app.js << 'EOF'
// Main application script
document.addEventListener('DOMContentLoaded', () => {
    console.log('App loaded');
});
EOF

# Create Python script
cat > /mnt/storage/scripts/backup.py << 'EOF'
#!/usr/bin/env python3
import os
import shutil
from datetime import datetime

# Simple backup script
source_dir = "/path/to/source"
backup_dir = f"/path/to/backup_{datetime.now().strftime('%Y%m%d')}"

shutil.copytree(source_dir, backup_dir)
print(f"Backup created: {backup_dir}")
EOF

# Create JSON configuration
cat > /mnt/storage/config/app-settings.json << 'EOF'
{
    "app_name": "MyApp",
    "version": "1.0.0",
    "settings": {
        "theme": "dark",
        "notifications": true
    }
}
EOF
```

### 2. Reading Files

**Read entire file:**
```bash
cat /mnt/storage/notes/meeting-2024-01-15.md
```

**Read specific lines (e.g., first 20 lines):**
```bash
head -20 /mnt/storage/reports/quarterly.txt
```

**Read last lines:**
```bash
tail -50 /mnt/storage/logs/app.log
```

**Read with line numbers:**
```bash
nl /mnt/storage/notes/ideas.md
```

**Search within files:**
```bash
grep -n "important" /mnt/storage/notes/*.md
grep -r "todo" /mnt/storage/
```

### 3. Listing Directory Contents

**List files in a directory:**
```bash
ls -la /mnt/storage/
```

**List with details:**
```bash
ls -lh /mnt/storage/projects/
```

**Tree view (if tree installed):**
```bash
tree /mnt/storage/
```

**List by modification time:**
```bash
ls -lt /mnt/storage/notes/ | head -20
```

**Count files:**
```bash
ls /mnt/storage/notes/ | wc -l
```

### 4. Updating and Appending Files

**Append to end of file:**
```bash
echo "" >> /mnt/storage/notes/meeting-2024-01-15.md
echo "## Follow-up Actions" >> /mnt/storage/notes/meeting-2024-01-15.md
echo "- [ ] Send summary to team" >> /mnt/storage/notes/meeting-2024-01-15.md
```

**Prepend to beginning of file:**
```bash
# Create new content
echo "# Updated: $(date)

$(cat /mnt/storage/notes/meeting-2024-01-15.md)" > /mnt/storage/notes/meeting-2024-01-15.md
```

**Replace specific line:**
```bash
# Replace line 5 with new content
sed -i '5s/.*/New content for line 5/' /mnt/storage/notes/file.md
```

**Insert after specific line:**
```bash
# Insert after line 3
sed -i '3a\New line inserted here' /mnt/storage/notes/file.md
```

**Find and replace text:**
```bash
# Replace all occurrences of "old" with "new"
sed -i 's/old/new/g' /mnt/storage/notes/file.md

# Replace only first occurrence per line
sed -i 's/old/new/' /mnt/storage/notes/file.md
```

### 5. Directory Organization

**Create subdirectories:**
```bash
mkdir -p /mnt/storage/{notes,projects,scripts,lists,reports,archives,websites}

# Create project-specific structure
mkdir -p /mnt/storage/projects/website/{css,js,images,templates}
mkdir -p /mnt/storage/projects/blog/{posts,drafts,published}
```

**Move files:**
```bash
mv /mnt/storage/temp/file.md /mnt/storage/notes/
mv /mnt/storage/draft.md /mnt/storage/projects/blog/drafts/
```

**Copy files:**
```bash
cp /mnt/storage/notes/template.md /mnt/storage/notes/meeting-2024-02-01.md
```

**Rename files:**
```bash
mv /mnt/storage/notes/old-name.md /mnt/storage/notes/new-name.md
```

**Delete files (be careful!):**
```bash
rm /mnt/storage/temp/temp-file.txt
```

**Delete empty directories:**
```bash
rmdir /mnt/storage/empty-folder/
```

**Delete directories with content:**
```bash
rm -r /mnt/storage/old-project/
```

## Best Practices

### File Naming
- Use lowercase with hyphens: `meeting-notes-2024-01-15.md`
- Include dates for temporal files: `report-2024-Q1.md`
- Be descriptive but concise
- Avoid spaces (use hyphens or underscores)

### Directory Structure
```
/mnt/storage/
├── notes/              # Quick notes, meeting notes, ideas
├── projects/           # Active projects with subfolders
│   ├── website/
│   ├── blog/
│   └── app/
├── scripts/            # Python, shell scripts
├── lists/              # Shopping lists, todo lists
├── reports/            # Generated reports, summaries
├── archives/           # Completed/finished projects
├── websites/           # Static HTML websites
│   ├── site-1/
│   ├── site-2/
│   └── css/            # Shared stylesheets
└── config/             # Configuration files
```

### Safety Guidelines
- Always use absolute paths (/mnt/storage/...)
- Verify directory exists before creating files
- Use quotes around paths with spaces
- Test destructive operations first (use echo to preview)
- Create backups before major changes

## Quick Reference

| Task | Command |
|------|---------|
| Create file | `echo "content" > /path/to/file` |
| Append to file | `echo "content" >> /path/to/file` |
| Read file | `cat /path/to/file` |
| List directory | `ls -la /path/to/dir` |
| Find text | `grep "search" /path/to/file` |
| Create directory | `mkdir -p /path/to/dir` |
| Move file | `mv /old/path /new/path` |
| Copy file | `cp /source/path /dest/path` |
| Delete file | `rm /path/to/file` |
| Find and replace | `sed -i 's/old/new/g' /path/to/file` |

## Examples for Sandy

When user asks:
- "Create a note about my project ideas" → Create markdown file in /mnt/storage/notes/
- "Write a Python script to backup files" → Create .py file in /mnt/storage/scripts/
- "Make a todo list for today" → Create markdown with checkboxes in /mnt/storage/lists/
- "Build me a simple website" → Create HTML/CSS/JS files in /mnt/storage/websites/
- "Save this code snippet" → Create file with appropriate extension in /mnt/storage/

Always organize files into appropriate subdirectories based on type and purpose.
