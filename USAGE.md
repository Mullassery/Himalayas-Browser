# Himalayas Browser - Usage Guide

Comprehensive guide to using Himalayas Browser and its features.

**Table of Contents**
- [Daily Usage](#daily-usage)
- [AI Features](#ai-features)
- [Document Management](#document-management)
- [Privacy & Security](#privacy--security)
- [Workspaces](#workspaces)
- [Keyboard Navigation](#keyboard-navigation)
- [Advanced Workflows](#advanced-workflows)
- [API & Automation](#api--automation)

---

## Daily Usage

### Opening URLs

```bash
# Open homepage
himalayas

# Open specific website
himalayas https://github.com

# Open multiple tabs
himalayas https://gmail.com https://calendar.google.com

# Open private window
himalayas --new-private-window

# Open in existing window
himalayas https://example.com  # (opens new tab if window exists)
```

### Tab Management

#### Creating & Closing

| Action | Shortcut | Menu |
|--------|----------|------|
| New tab | `Ctrl+T` | File → New Tab |
| Close tab | `Ctrl+W` | File → Close Tab |
| Reopen closed | `Ctrl+Shift+T` | File → Reopen Tab |
| Close window | `Ctrl+Q` | File → Quit |

#### Switching & Moving

| Action | Shortcut | Menu |
|--------|----------|------|
| Next tab | `Ctrl+Tab` | Window → Next Tab |
| Previous tab | `Ctrl+Shift+Tab` | Window → Previous Tab |
| Jump to tab N | `Ctrl+1` to `Ctrl+8` | — |
| Last tab | `Ctrl+9` | — |
| Move tab left | `Ctrl+Shift+PageUp` | — |
| Move tab right | `Ctrl+Shift+PageDown` | — |
| Move to new window | `Ctrl+Shift+N` | — |

#### Tab Organization

| Action | Shortcut | Menu |
|--------|----------|------|
| Pin tab | `Ctrl+Shift+P` | Right-click → Pin Tab |
| Mute tab | `Ctrl+Shift+M` | Right-click → Mute |
| Archive tab | `Ctrl+Alt+A` | Right-click → Archive |
| Duplicate tab | `Ctrl+Alt+D` | Right-click → Duplicate |

### Browsing

#### Navigation

| Action | Shortcut | Menu |
|--------|----------|------|
| Go back | `Alt+←` or Backspace | Back button |
| Go forward | `Alt+→` | Forward button |
| Go home | `Alt+Home` | Home button |
| Open history | `Ctrl+H` | History menu |
| Search history | `Ctrl+H` then type | — |

#### Searching

| Action | Method |
|--------|--------|
| Find in page | `Ctrl+F` → type |
| Find next | `Enter` or `Ctrl+G` |
| Find previous | `Shift+Enter` or `Ctrl+Shift+G` |
| Regular expression search | `Ctrl+F` → enable ".*" |
| Case sensitive | `Ctrl+F` → toggle "Aa" |

#### Page Interaction

| Action | Shortcut | Purpose |
|--------|----------|---------|
| Scroll down | `Space` or `Page Down` | Read down |
| Scroll up | `Shift+Space` or `Page Up` | Read up |
| Top of page | `Home` | Jump to top |
| Bottom of page | `End` | Jump to bottom |
| Reload page | `Ctrl+R` | Refresh content |
| Hard reload | `Ctrl+Shift+R` | Clear cache + reload |

---

## AI Features

### AI Assistant

**Activation**: `Ctrl+Space` (Windows/Linux) or `Cmd+Space` (macOS)

Opens AI assistant panel with context-aware suggestions.

#### Available Commands

**Text Analysis**
```
Summarize      → Brief summary of content
Explain        → Explain in simpler terms
Translate      → Translate to another language
Define         → Look up word definition
Extract        → Pull structured data
```

**Content Generation**
```
Rewrite        → Rephrase and improve
Generate Code  → Write code snippet
Generate Tests → Create test cases
Generate Docs  → Write documentation
```

**Verification**
```
Fact Check     → Verify claims
Check Safety   → Scan for suspicious content
Find Issues    → Identify problems
```

**Navigation**
```
Find Related   → Find related topics
Search Web     → Web search for topic
Open Links     → Find and open links
```

#### Using AI Assistant

1. **Highlight text** (optional)
2. **Press `Ctrl+Space`** (or `Cmd+Space` on macOS)
3. **AI assistant opens** showing relevant commands
4. **Select command** or type to search
5. **Results appear** in side panel

#### Example Workflows

**Summarize Article**
```
1. Open article
2. Ctrl+Space
3. Select "Summarize"
4. Read 3-line summary in side panel
5. Click "Expand" for full summary
```

**Translate Page**
```
1. Open page in foreign language
2. Ctrl+Space
3. Select "Translate" 
4. Choose target language
5. Page refreshes with translation
```

**Extract Information**
```
1. Open form, table, or structured content
2. Highlight relevant section
3. Ctrl+Space
4. Select "Extract"
5. Structured data appears in panel
```

### Page Intelligence

**Right-click on page** → AI menu options

#### Smart Context Menu

Menu changes based on what you've selected:

**On Text**
```
Summarize      → Condense selected text
Explain        → Break down concept
Translate      → Translate selection
Fact Check     → Verify claims
Save to Notes  → Save for later
```

**On Image**
```
Describe       → What's in the image
Identify       → Name objects
OCR Text       → Extract text from image
Reverse Search → Find similar images
```

**On Link**
```
Preview        → Show link target without opening
Summary        → Summarize linked page
Safe Check     → Is link safe to click
Open → New Tab → Open in background tab
```

**On Code**
```
Explain        → What does this code do
Debug          → Find issues
Optimize       → Improve performance
Test Generate  → Write tests
Security Check → Find vulnerabilities
```

**On PDF**
```
Summarize      → Overview of PDF
Extract        → Pull tables and data
OCR            → Extract text from scans
Compare        → Find differences between versions
```

---

## Document Management

### Viewing Documents

Himalayas supports:
- **PDF** (.pdf)
- **Word** (.docx)
- **Excel** (.xlsx)
- **PowerPoint** (.pptx)
- **Text** (.txt, .rtf)
- **OpenDocument** (.odt, .ods)

#### Opening Documents

```bash
# Open from web
himalayas https://example.com/report.pdf

# Open from disk
himalayas ~/Documents/report.pdf

# Or drag-and-drop into browser
```

### PDF Interactions

#### Reading

| Action | Shortcut |
|--------|----------|
| Next page | `Space` or `→` |
| Previous page | `Backspace` or `←` |
| First page | `Home` |
| Last page | `End` |
| Go to page N | `G` then type page number |
| Zoom in | `+` or `Ctrl+Scroll` |
| Zoom out | `-` or `Ctrl+Scroll` |
| Fit width | `W` |
| Fit page | `Ctrl+0` |
| Continuous mode | `Shift+Enter` |

#### Annotation

| Action | How | Use Case |
|--------|-----|----------|
| Highlight | Click color box → click text | Mark important parts |
| Underline | Right-click → Underline | Emphasize points |
| Circle | Right-click → Circle | Draw attention to detail |
| Note | Right-click → Add Note | Add comments |
| Arrow | Drawing tool → drag | Point out specific area |
| Free draw | Free drawing tool | Sketch or mark up |

#### Extraction

| Action | How |
|--------|-----|
| Extract text | Right-click → Extract Text |
| Extract tables | Right-click → Extract Tables |
| Extract all | Right-click → Extract All Data |
| Export as JSON | Right-click → Export |

### Document Search

#### Within Document

```bash
Ctrl+F              # Find in PDF
Type search term    # Search text
Enter              # Next match
Shift+Enter        # Previous match
```

#### Advanced Search

```bash
regex:pattern      # Regular expression
case:sensitive     # Case sensitive
whole:word         # Whole word only
```

---

## Privacy & Security

### Privacy Features

All enabled by default:

```
✅ Private browsing (no history saved)
✅ Cookie blocking (first-party only)
✅ Tracking prevention (pixels, trackers blocked)
✅ Fingerprint resistance (randomized)
✅ DNS over HTTPS (secure lookups)
✅ Ephemeral storage (cleared on exit)
```

### Check Privacy Status

| Feature | How to Check |
|---------|--------------|
| Cookies | Ctrl+Shift+P → "show cookies" |
| Trackers | Ctrl+Shift+P → "show trackers" |
| Permissions | Ctrl+Shift+P → "manage permissions" |
| Fingerprinting | Ctrl+Shift+P → "fingerprint status" |
| Storage | Developer Tools → Application tab |

### Managing Permissions

#### View Active Permissions

```bash
Ctrl+Shift+P  # Open command palette
"permissions" # Search for permissions
```

Shows:
- Location access
- Camera/Microphone
- Notification access
- Storage access
- Clipboard access

#### Grant Permission

When site requests permission:
1. Dialog appears
2. Click "Allow" or "Block"
3. Set expiration (auto-expires after duration)

#### Revoke Permission

```bash
Ctrl+Shift+P          # Open command palette
"revoke permissions"  # Search
Select site           # Choose which site
Select permission     # Choose what to revoke
Confirm              # Permission revoked
```

### Clear Data

#### Quick Clear

```bash
Ctrl+Shift+P     # Open command palette
"clear cache"    # Search for clear
Select options:
  ☑ Browsing history
  ☑ Cookies
  ☑ Cache
  ☑ Local storage
  ☑ Temporary files
Click Clear      # Confirm
```

#### On Exit

Enable in config:

```toml
[privacy]
auto_cleanup = true    # Automatically clear on exit
```

---

## Workspaces

### What are Workspaces?

Separate browsing contexts for different activities:

- **Work**: Professional tasks, emails, documents
- **Personal**: Social media, entertainment
- **Research**: Articles, academic papers, PDFs
- **Shopping**: Stores, wishlists, compare prices

Each workspace has:
- Separate tabs
- Independent history
- Isolated permissions
- Private data storage

### Creating Workspaces

#### Via Command Palette

```bash
Ctrl+Shift+P
"create workspace"
Enter name:  Research
Press Enter
```

#### Via Menu

```
Workspace → New Workspace
Name: Personal
Create
```

### Switching Workspaces

#### Keyboard

```bash
Ctrl+Shift+W    # Open workspace switcher
Type name       # Find workspace
Enter          # Switch
```

#### Mouse

```
Workspace menu → Select workspace
```

#### Keyboard Cycling

```bash
Ctrl+Alt+Right  # Next workspace
Ctrl+Alt+Left   # Previous workspace
```

### Managing Workspaces

#### Rename

```bash
Ctrl+Shift+P
"rename workspace"
Enter new name
```

#### Delete

```bash
Ctrl+Shift+P
"delete workspace"
Confirm deletion
```

#### Archive

```bash
Ctrl+Shift+P
"archive workspace"
# Workspace hidden but not deleted
```

#### Move Tabs Between Workspaces

```bash
Right-click tab
"Move to workspace"
Select destination workspace
```

### Workspace Tips

```
💡 Create workspace for each project
💡 Use workspace names as context (e.g., "Project Alpha")
💡 Different permissions per workspace
💡 Faster context switching (no tab clutter)
💡 Archive finished workspaces (keep history)
```

---

## Keyboard Navigation

### Complete Reference

#### Browser Control

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| New window | Ctrl+N | Cmd+N |
| New tab | Ctrl+T | Cmd+T |
| Close tab | Ctrl+W | Cmd+W |
| Close window | Ctrl+Q | Cmd+Q |
| Reopen tab | Ctrl+Shift+T | Cmd+Shift+T |
| Quit browser | Ctrl+Q | Cmd+Q |

#### Navigation

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Back | Alt+← | Cmd+[ |
| Forward | Alt+→ | Cmd+] |
| Home | Alt+Home | Cmd+Home |
| Reload | Ctrl+R | Cmd+R |
| Hard reload | Ctrl+Shift+R | Cmd+Shift+R |
| Stop | Escape | Escape |
| Focus address bar | Ctrl+L | Cmd+L |

#### Tab Navigation

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Next tab | Ctrl+Tab | Cmd+} |
| Previous tab | Ctrl+Shift+Tab | Cmd+{ |
| Tab 1-8 | Ctrl+1 to 8 | Cmd+1 to 8 |
| Last tab | Ctrl+9 | Cmd+9 |

#### Page Functions

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Find | Ctrl+F | Cmd+F |
| Print | Ctrl+P | Cmd+P |
| Save | Ctrl+S | Cmd+S |
| View source | Ctrl+U | Cmd+U |
| Zoom in | Ctrl++ | Cmd++ |
| Zoom out | Ctrl+- | Cmd+- |
| Reset zoom | Ctrl+0 | Cmd+0 |
| Full screen | F11 | Ctrl+Cmd+F |

#### Developer

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| DevTools | F12 | F12 |
| Inspector | Ctrl+Shift+C | Cmd+Shift+C |
| Console | Ctrl+Shift+J | Cmd+Shift+J |
| Debugger | Ctrl+Shift+I | Cmd+Shift+I |

#### AI & Search

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| AI Assistant | Ctrl+Space | Cmd+Space |
| Command palette | Ctrl+Shift+P | Cmd+Shift+P |
| Search tabs | Ctrl+Shift+A | Cmd+Shift+A |
| Summarize | Ctrl+Shift+S | Cmd+Shift+S |

### Vim Mode (Optional)

Enable in config:

```toml
[keyboard]
vim_mode = true
```

Navigation:
```
j, k       → Down, Up
h, l       → Left, Right
gg         → Top of page
G          → Bottom of page
/          → Search
n, N       → Next, Previous match
gt, gT     → Next, Previous tab
```

---

## Advanced Workflows

### Workflow 1: Research Project

**Goal**: Collect and organize research for a project

```
1. Create workspace "Project Alpha"
   Ctrl+Shift+P → "create workspace"

2. Gather sources
   Ctrl+T → Open first source
   Ctrl+T → Open second source
   Ctrl+T → Open third source

3. Annotate documents
   For each source:
   - Highlight key passages
   - Add notes to highlights
   - Extract data (Right-click → Extract)

4. Organize findings
   Ctrl+Space → Summarize each page
   Save summaries to notes

5. Export results
   Ctrl+Shift+P → "export workspace"
   Get ZIP file with all data

6. Archive workspace
   Ctrl+Shift+P → "archive workspace"
```

### Workflow 2: Government Form Filing

**Goal**: Fill and submit government forms with AI assistance

```
1. Create workspace "Government Task"

2. Authenticate
   Open government portal
   Log in with credentials

3. AI-assisted filling
   Navigate to form
   Ctrl+Space → "extract fields"
   AI identifies form fields
   Ctrl+Space → "auto-fill"

4. Verification
   Ctrl+Space → "fact check"
   Verify all information

5. Sign document
   Ctrl+Space → "digital sign"
   Use eSign (if integrated)

6. Submit
   Click submit
   Save receipt

7. Track application
   Ctrl+Space → "check status"
   Set reminder for follow-up
```

### Workflow 3: Document Comparison

**Goal**: Compare two versions of a document

```
1. Create workspace "Document Review"

2. Open both documents
   Tab 1: Original version (PDF)
   Tab 2: Updated version (PDF)

3. Compare
   Ctrl+Space (on Tab 1) → "compare"
   Select Tab 2
   Differences highlighted

4. Review changes
   Navigate through differences
   Annotate changes as needed

5. Approve or reject
   Ctrl+Space → "approve changes"
   Or: "reject changes"

6. Export diff
   Ctrl+Shift+P → "export comparison"
   Get detailed change report
```

### Workflow 4: Batch Processing

**Goal**: Process multiple similar items (forms, documents, etc.)

```
1. Create workspace "Batch Processing"

2. Queue items
   Ctrl+T × 10 (open 10 tabs with items)

3. Create macro
   Ctrl+Alt+J → Start recording
   Process first item:
     - Open item
     - Fill required fields (or AI does it)
     - Verify
     - Submit
   Ctrl+Alt+J → Stop recording
   Name: "process_item"

4. Batch process
   For each remaining tab:
     Click tab
     Ctrl+Shift+R (replay macro)
     Verify results
     Continue

5. Report
   Ctrl+Shift+P → "generate report"
   Get summary of processed items
```

---

## API & Automation

### Headless Mode

Run without GUI for automation:

```bash
himalayas --headless https://example.com

# With API server
himalayas --headless --api-server 8000

# Open in background
himalayas --headless &
```

### REST API

Start API server:

```bash
himalayas --api-server 8000
```

#### Available Endpoints

**Tabs**
```bash
# List tabs
curl http://localhost:8000/tabs

# Get tab details
curl http://localhost:8000/tabs/{id}

# Create tab
curl -X POST http://localhost:8000/tabs \
  -d '{"url":"https://example.com"}'

# Close tab
curl -X DELETE http://localhost:8000/tabs/{id}
```

**Navigation**
```bash
# Navigate to URL
curl -X POST http://localhost:8000/navigate \
  -d '{"tab_id":1,"url":"https://example.com"}'

# Go back
curl -X POST http://localhost:8000/back \
  -d '{"tab_id":1}'

# Go forward
curl -X POST http://localhost:8000/forward \
  -d '{"tab_id":1}'
```

**Content**
```bash
# Get page content
curl http://localhost:8000/tabs/{id}/content

# Execute JavaScript
curl -X POST http://localhost:8000/tabs/{id}/execute \
  -d '{"script":"document.title"}'

# Get page screenshot
curl http://localhost:8000/tabs/{id}/screenshot
```

**AI Operations**
```bash
# Summarize
curl -X POST http://localhost:8000/ai/summarize \
  -d '{"tab_id":1}'

# Translate
curl -X POST http://localhost:8000/ai/translate \
  -d '{"tab_id":1,"language":"es"}'

# Extract entities
curl -X POST http://localhost:8000/ai/extract \
  -d '{"tab_id":1}'
```

### Python Integration

```python
import requests
import json

class HimalayasClient:
    def __init__(self, host="localhost", port=8000):
        self.base_url = f"http://{host}:{port}"
    
    def open_url(self, url):
        """Open URL in new tab"""
        response = requests.post(
            f"{self.base_url}/tabs",
            json={"url": url}
        )
        return response.json()
    
    def summarize(self, tab_id):
        """Summarize page"""
        response = requests.post(
            f"{self.base_url}/ai/summarize",
            json={"tab_id": tab_id}
        )
        return response.json()
    
    def extract_data(self, tab_id):
        """Extract structured data"""
        response = requests.post(
            f"{self.base_url}/ai/extract",
            json={"tab_id": tab_id}
        )
        return response.json()

# Usage
client = HimalayasClient()

# Open page
tab = client.open_url("https://example.com")
tab_id = tab["id"]

# Summarize
summary = client.summarize(tab_id)
print(summary["text"])

# Extract data
data = client.extract_data(tab_id)
print(json.dumps(data, indent=2))
```

### Automation Examples

#### Scrape and Summarize News

```bash
#!/bin/bash

himalayas --headless --api-server 8000 &
SERVER_PID=$!

# Open news page
curl -X POST http://localhost:8000/tabs \
  -d '{"url":"https://news.example.com"}' > tab.json

TAB_ID=$(jq .id tab.json)

# Wait for load
sleep 2

# Summarize
curl -X POST http://localhost:8000/ai/summarize \
  -d "{\"tab_id\":$TAB_ID}" > summary.json

# Display
jq .summary summary.json

kill $SERVER_PID
```

#### Form Automation

```python
from himalaya import HimalayasClient
import time

client = HimalayasClient()

# Open form
tab = client.open_url("https://forms.example.com")
tab_id = tab["id"]

time.sleep(2)  # Wait for load

# Fill form (AI-assisted)
form_data = {
    "name": "John Doe",
    "email": "john@example.com",
    "subject": "Inquiry"
}

for field, value in form_data.items():
    # Find and fill field
    client.execute(tab_id, f'''
        document.querySelector('[name="{field}"]').value = "{value}";
    ''')

# Submit
client.execute(tab_id, 'document.querySelector("form").submit()')
```

---

## Tips & Best Practices

### Productivity

```
1. Use workspaces for different contexts
2. Master keyboard shortcuts (no mouse)
3. Use command palette for quick access
4. Create macros for repetitive tasks
5. Set up AI assistant shortcuts
```

### Security

```
1. Always use private window (default)
2. Review and revoke permissions regularly
3. Monitor active sessions (Ctrl+Shift+P → "sessions")
4. Export and verify activity logs
5. Use strong passwords
```

### Performance

```
1. Close unused tabs to free memory
2. Archive old workspaces
3. Clear cache regularly
4. Disable auto-play of media
5. Use low-memory profile on old devices
```

---

## Troubleshooting

### Issue: Slow Performance

**Solution**:
1. Close unused tabs
2. Lower device profile (Ctrl+Shift+P → "device profile")
3. Disable extensions (if any)
4. Clear cache (Ctrl+Shift+P → "clear cache")

### Issue: AI Assistant Not Working

**Solution**:
1. Check internet connection
2. Verify AI is enabled (config: `enable_local_models = true`)
3. Check permissions (Ctrl+Shift+P → "permissions")
4. Restart browser

### Issue: PDF Not Displaying

**Solution**:
1. Download PDF locally
2. Open with file:// URL
3. Check if PDF is corrupted (try another tool)
4. Enable JavaScript (if required by PDF)

### Issue: Form Not Auto-Filling

**Solution**:
1. Check form uses standard HTML
2. Verify AI extraction sees fields (Ctrl+Space → "extract")
3. Try manual fill
4. Use keyboard Tab navigation

---

**More help**: See [GETTING_STARTED.md](./GETTING_STARTED.md) or visit [GitHub Issues](https://github.com/Mullassery/Himalayas-Browser/issues)

🏔️ **Reaching the peak of autonomous computing**
