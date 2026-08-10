# Keyboard & Trackpad Support Specification

**Version**: 0.2.0 (Planned Phase 6)  
**Status**: Design specification  
**Target Release**: Q4 2026

This document is the implementation-level spec for keyboard and trackpad input. For the broader interface vision it fits into (adaptive UI, AI workspace, sidebar, workspaces, design language, etc.), see [UI_UX_VISION.md](./UI_UX_VISION.md).

---

## Vision

Himalayas Browser with **100% keyboard accessibility** and **deep multi-touch trackpad integration** across Windows, macOS, and Linux. Every function accessible without a mouse. AI understands user intent from gestures.

---

## Core Principles

1. **Mouse Optional** - Every feature accessible from keyboard
2. **Keyboard First** - Power users prioritized
3. **Touch Friendly** - Gesture-aware on trackpads/touchscreens
4. **AI-Aware** - Intent recognition, not just input replay
5. **Customizable** - User-definable shortcuts and gestures
6. **Accessible** - Full screen reader support
7. **Low Latency** - <1 frame (16ms @ 60Hz)
8. **Works Offline** - No cloud dependency for keybindings
9. **Cross-Platform** - Consistent experience (with native optimizations)
10. **Developer Friendly** - Extensible API for plugins

---

## Full Keyboard Navigation

### Every UI Element Reachable

- ✅ Address bar
- ✅ Tabs (create, close, move, switch)
- ✅ Sidebar (bookmarks, history, downloads)
- ✅ AI panel
- ✅ Developer Tools
- ✅ Settings
- ✅ Split view
- ✅ Reader mode
- ✅ Picture-in-picture
- ✅ Media controls
- ✅ Extensions panel
- ✅ Workspace switcher

### Tab Navigation
```
Tab/Shift+Tab           → Move focus forward/backward
Arrow Keys              → Navigate menu items
Enter                   → Activate focused item
Space                   → Toggle/expand focused item
Escape                  → Close menu/dialog
```

---

## Universal Command Palette

**Activation**: `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (macOS)

### Searchable Functions

**Navigation**
- Open tab (exact match)
- Go to URL
- Find in page
- Show history
- Open bookmarks
- Close tab
- Restore tab
- Move tab

**Document**
- Open file
- Save page
- Print
- Reader mode
- PDF generation
- Screenshot
- Inspect element

**AI**
- Summarize page
- Explain selection
- Translate
- Fact check
- Generate tests
- Debug code
- Optimize code
- Extract entities

**System**
- Clear cache
- Clear history
- Clear cookies
- Reset settings
- Export data
- Import data

**Workspace**
- Create workspace
- Switch workspace
- Rename workspace
- Move tab to workspace
- Archive workspace

**Developer**
- Open DevTools
- Open Console
- Open Inspector
- Open Network tab
- Open Performance tab
- Record performance
- Toggle breakpoint

### Features
- Fuzzy search matching
- Command aliases
- Recent commands list
- Search history
- Icon display
- Keyboard-only operation

---

## Intelligent Keyboard Shortcuts

### Shortcut Types

**Single Key**
```
h        → Home
j        → Jump to next
k        → Jump to previous
/        → Find
?        → Help
```

**Double Key**
```
g h      → Go home
g d      → Go downloads
g b      → Go bookmarks
g s      → Go settings
```

**Key Sequences**
```
Ctrl K   → Quick search
Ctrl L   → Focus address bar
Ctrl T   → New tab
Ctrl W   → Close tab
```

**Chord Shortcuts**
```
Ctrl+Shift+N     → New private window
Ctrl+Alt+T       → Open terminal (if available)
Ctrl+Shift+Del   → Clear browsing data
```

### Default Bindings

**Tab Management**
| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| New tab | Ctrl+T | Cmd+T |
| Close tab | Ctrl+W | Cmd+W |
| Reopen tab | Ctrl+Shift+T | Cmd+Shift+T |
| Next tab | Ctrl+Tab | Cmd+} |
| Previous tab | Ctrl+Shift+Tab | Cmd+{ |
| Move tab left | Ctrl+Shift+PageUp | Cmd+Shift+← |
| Move tab right | Ctrl+Shift+PageDown | Cmd+Shift+→ |
| Pin tab | Ctrl+Shift+K | Cmd+Shift+K |

**Navigation**
| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Back | Alt+← or Backspace | Cmd+[ |
| Forward | Alt+→ | Cmd+] |
| Reload | Ctrl+R | Cmd+R |
| Hard reload | Ctrl+Shift+R | Cmd+Shift+R |
| Focus address bar | Ctrl+L | Cmd+L |
| Search tabs | Ctrl+Shift+A | Cmd+Shift+A |

**Page Functions**
| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Summarize | Ctrl+Shift+S | Cmd+Shift+S |
| Find | Ctrl+F | Cmd+F |
| Print | Ctrl+P | Cmd+P |
| Save page | Ctrl+S | Cmd+S |
| View source | Ctrl+U | Cmd+U |
| DevTools | F12 | F12 |
| Inspector | Ctrl+Shift+C | Cmd+Shift+C |

---

## User Customization

### Import/Export
```yaml
# Export format (YAML/JSON)
shortcuts:
  - name: "Go Home"
    keys: [Ctrl, H]
    action: "navigate_home"
    mode: "all"
  - name: "Summarize"
    keys: [Ctrl, Shift, S]
    action: "ai_summarize"
    modes: ["normal", "power_user"]
```

Users can:
- ✅ Redefine every shortcut
- ✅ Export custom profiles
- ✅ Import community profiles
- ✅ Share profiles with team
- ✅ Per-workspace customization
- ✅ Fallback to defaults

### Keyboard Profiles

**Built-in Profiles**
- Standard (balanced)
- Power User (vim-like)
- Developer (IDE-like)
- Gaming (low latency)
- Mac Native (macOS conventions)
- Windows Native (Windows conventions)
- Linux Native (Linux conventions)
- Vim Mode (full vim navigation)
- Emacs Mode (emacs bindings)
- IDE Layout (VSCode/JetBrains)

**Modes**
- Normal Mode (default)
- Power User (advanced shortcuts)
- Developer (DevTools shortcuts)
- Accessibility (high contrast, larger fonts)
- Presentation (simplified UI)
- Minimal (bare essentials)

Each mode exposes different shortcut sets.

---

## Vim Mode

Complete vim-like navigation without modal interface complexity.

### Navigation
```
j            → Scroll down
k            → Scroll up
gg           → Go to top
G            → Go to bottom
H            → Go to home
```

### Search & Movement
```
/text        → Find forward
?text        → Find backward
n            → Next match
N            → Previous match
f            → Find element (click equivalent)
F            → Find element (reverse)
```

### Tab Operations
```
gt           → Next tab
gT           → Previous tab
gn           → New tab
gx           → Close tab
gr           → Reload tab
```

### Page Operations
```
y            → Copy URL
d            → Download link
r            → Reload
b            → Bookmarks menu
h            → History
t            → New tab
```

### Full Mode Toggle
```
Escape       → Toggle vim mode
:set novimode → Disable vim mode
```

---

## Emacs Mode

Native Emacs keybindings for text editing and navigation.

### Navigation
```
Ctrl+F       → Forward char
Ctrl+B       → Backward char
Ctrl+N       → Next line
Ctrl+P       → Previous line
Meta+F       → Forward word
Meta+B       → Backward word
Ctrl+A       → Beginning of line
Ctrl+E       → End of line
```

### Editing
```
Ctrl+K       → Kill line
Ctrl+Y       → Yank (paste)
Ctrl+_       → Undo
Meta+D       → Kill word
Meta+Backspace → Backward kill word
```

### Search
```
Ctrl+S       → Isearch forward
Ctrl+R       → Isearch backward
Meta+%       → Query replace
```

### Registers & Kill Ring
```
Ctrl+Space   → Set mark
Ctrl+W       → Kill region
Meta+W       → Copy region
Ctrl+X Ctrl+X → Exchange point and mark
```

---

## Keyboard Macros

Users can record and replay workflows.

### Recording
```
Ctrl+Alt+J   → Start recording macro
[perform actions]
Ctrl+Alt+J   → Stop recording
Ctrl+Alt+K   → Play last macro
```

### Example Workflow
```
1. Ctrl+Shift+P         → Open command palette
2. "Open Dashboard"     → Find and select
3. Enter                → Execute
4. Ctrl+R               → Refresh
5. Ctrl+Alt+D           → Download CSV
6. [navigate to rename]
7. F2                   → Rename
8. Ctrl+X               → Archive

[Save as: daily_report_macro]
[Replay with: Ctrl+Shift+R]
```

### Macro Management
- Save/load macros
- Macro library
- Conditional macros
- Time-delayed steps
- Variable substitution

---

## AI Keyboard Assistant

**Activation**: `Ctrl+Space` or `Cmd+Space`

AI panel appears instantly at cursor, understanding context.

### Commands
```
Summarize       → Summarize selected text or page
Explain         → Explain with AI
Translate       → Translate to specified language
Extract         → Extract structured data
Rewrite         → Rephrase/improve text
Generate Code   → Generate code snippet
Compare         → Compare two documents
Fact Check      → Verify claims
Define          → Define term
Expand          → Expand abbreviation
```

### Context-Aware
- Analyzes surrounding content
- Offers relevant commands
- Learns from user behavior
- Suggests next steps

---

## Context-Aware Shortcuts

Shortcut behavior changes based on active context.

### Reading PDF
```
Space           → Next page
Shift+Space     → Previous page
Ctrl+F          → Find in PDF
Ctrl+P          → Print PDF
Ctrl+S          → Save PDF
Ctrl+E          → Extract text
```

### Playing Video
```
Space           → Play/Pause
J               → Rewind 10s
K               → Forward 10s
M               → Mute
F               → Fullscreen
C               → Toggle captions
< / >           → Slower/Faster
```

### In Terminal/Shell
```
Space           → Insert character (not search)
Ctrl+C          → Interrupt (not copy)
Ctrl+L          → Clear screen (not copy)
Ctrl+D          → EOF (not delete)
```

### In AI Chat
```
Space           → Normal typing (not next page)
Enter           → Send message (not activate)
Ctrl+K          → Previous message
Ctrl+J          → Next message
Escape          → Close chat
```

### In Code Editor
```
Tab             → Indent (not focus change)
Ctrl+/          → Comment toggle
Ctrl+G          → Go to line
F11             → Fullscreen editor
Ctrl+F          → Find in file
```

---

## Keyboard-Driven Tab Management

Everything accessible without mouse.

### Operations
```
Ctrl+T          → Create tab
Ctrl+W          → Close tab
Ctrl+Shift+T    → Restore tab
Ctrl+Tab        → Next tab
Ctrl+Shift+Tab  → Previous tab
Ctrl+Shift+M    → Mute tab
Ctrl+Shift+K    → Pin tab
Ctrl+Alt+S      → Suspend tab
Ctrl+Alt+R      → Restore tab
Ctrl+Shift+G    → Group tabs
Ctrl+Alt+G      → Manage groups
Ctrl+Shift+F    → Fullscreen tab
Ctrl+Alt+D      → Duplicate tab
```

### Tab Search
```
Ctrl+Shift+A    → Search open tabs
[Type tab name]
[Select tab]
Enter           → Switch to tab
```

### Tab Arrangement
```
Ctrl+Shift+Left  → Move tab left
Ctrl+Shift+Right → Move tab right
Ctrl+Shift+Up    → Move tab to new window
Ctrl+Shift+Down  → Move tab to split
```

---

## Keyboard Workspace Control

Manage workspaces entirely from keyboard.

### Operations
```
Ctrl+Shift+N    → Create workspace
Ctrl+Shift+W    → Switch workspace
Ctrl+Shift+R    → Rename workspace
Ctrl+Shift+D    → Delete workspace
Ctrl+Shift+M    → Move tab to workspace
Ctrl+Alt+Left   → Previous workspace
Ctrl+Alt+Right  → Next workspace
```

### Workspace Search
```
Ctrl+Shift+W    → Open workspace switcher
[Type workspace name]
Enter           → Switch
```

---

## Omnibox (Keyboard Only)

Unified search supporting everything.

### Activation
```
Ctrl+L          → Focus omnibox
Ctrl+K          → Quick search
```

### Supported Searches
- **History** - Recent pages
- **Bookmarks** - Saved pages
- **Tabs** - Open tabs search
- **AI Search** - Semantic search
- **Calculator** - Math operations
- **Unit Conversion** - Temperature, distance, etc.
- **Clipboard History** - Recent copies
- **Commands** - Browser commands
- **Recently Closed** - Restore pages

### Examples
```
"amazon laptop"         → Search web
"!w python tutorial"    → Wikipedia search
"!yt music"             → YouTube search
"5 * 3 + 2"             → Calculator (= 17)
"100 km in miles"       → Unit conversion
"weather london"        → Weather lookup
"time tokyo"            → World time
"define serendipity"    → Dictionary
":config"               → Open settings
"@gmail compose"        → Gmail compose
```

---

## Global Browser Hotkeys

Work even when browser window is not focused (OS permission dependent).

### Quick Actions
```
Ctrl+Alt+C      → Quick capture (screenshot)
Ctrl+Alt+N      → Quick note (open notepad)
Ctrl+Alt+S      → Quick search (selected text)
Ctrl+Alt+T      → Quick translate
Ctrl+Alt+O      → Quick OCR
Ctrl+Alt+A      → AI assistant
Ctrl+Alt+B      → Open browser
```

### Platform-Specific
**Windows**: Registry configuration for global hotkeys  
**macOS**: Accessibility preferences required  
**Linux**: Window manager keybinding integration (configurable)

---

## Trackpad Integration

### Supported Hardware

**Apple Force Touch Trackpads**
- Force sensing
- Haptic feedback
- Pressure detection
- Multi-touch gestures

**Windows Precision Touchpads**
- Windows Precision standard
- Gesture support
- Haptic feedback (modern devices)

**Linux libinput** (including lightweight Linux-based laptops, which use the same protocol)
- Standard libinput protocol
- Gesture support
- Multi-touch detection

---

## Gesture Engine

Support for multi-touch gestures based on fingers used.

### One Finger Gestures
```
Single tap      → Click
Double tap      → Double click
Tap & hold      → Right-click menu
Swipe left      → Back
Swipe right     → Forward
Swipe up        → Scroll up
Swipe down      → Scroll down
```

### Two Finger Gestures
```
Two-finger scroll       → Vertical/horizontal scroll
Two-finger pinch in     → Zoom out
Two-finger pinch out    → Zoom in
Two-finger rotate       → Rotate (images/PDFs)
Two-finger swipe left   → Close tab
Two-finger swipe right  → Open tab
Two-finger tap          → Secondary click
```

### Three Finger Gestures
```
Three-finger swipe up     → Workspace/mission control
Three-finger swipe down   → Show all windows
Three-finger swipe left   → Previous workspace
Three-finger swipe right  → Next workspace
Three-finger tap          → Open AI panel
```

### Four Finger Gestures
```
Four-finger swipe left    → Previous workspace
Four-finger swipe right   → Next workspace
Four-finger swipe up      → Activity view
Four-finger swipe down    → Show desktop
Four-finger tap           → Split screen
```

### Five Finger Gestures
```
Five-finger pinch         → Minimize window
Five-finger spread        → Show all windows
```

---

## Force Touch (Apple)

Pressure-based interactions (Apple trackpads/Magic Mouse).

### Actions
```
Light press (force < 40%)      → Preview link
Medium press (force 40-70%)    → Show dictionary
Hard press (force > 70%)       → AI explanation
Very hard (force > 90%)        → Translate
```

### Context-Specific
- **Links**: Preview page content
- **Words**: Dictionary popup
- **Images**: Metadata display
- **Code**: Open documentation
- **PDF**: Open annotation panel

---

## Swipe Gestures

### Edge Swipes

**Swipe from Left Edge**
```
Single swipe            → Go back
Double swipe            → Go back 2 pages
Slow swipe              → Reveal sidebar
```

**Swipe from Right Edge**
```
Single swipe            → Open AI panel
Double swipe            → Open sidebar
Slow swipe              → Show history
```

**Swipe from Top Edge**
```
Single swipe            → Open command palette
Double swipe            → Open settings
Slow swipe              → Hide UI
```

**Swipe from Bottom Edge**
```
Single swipe            → Show downloads
Double swipe            → Show history
Slow swipe              → Show bookmarks
```

---

## Smart Gesture Recognition

AI differentiates between intentional and accidental touches.

### Differentiation
```
Fast swipe              → Intentional action
Slow swipe              → Adjustment/fine-tuning
Decisive drag           → Intentional movement
Tentative drag          → Exploratory (preview)
Palm contact            → Ignore (resting)
Thumb contact           → Ignore (holding device)
Finger hover            → Preview (no click)
```

### Learning
- Learns user gesture patterns
- Adapts sensitivity per user
- Context-aware thresholds
- Age/accessibility profile aware

---

## Haptic Feedback

Supported devices provide tactile confirmation.

### Events
```
Tab closed                      → Brief double vibration
Download complete              → Rising vibration
AI response ready              → Single strong pulse
Gesture recognized             → Soft double tap
Screenshot captured            → Triple vibration
Notification received          → Single long vibration
Permission granted/denied      → Distinct haptic
Gesture not recognized         → Error vibration
```

### Intensity Control
- Volume-like haptic control
- User preference settings
- Per-event customization
- Accessibility profiles

---

## Multi-Touch AI

AI understands gesture intent, not just replay.

### Drawing Recognition
```
Circle gesture              → "Explain this"
Rectangle selection         → "Extract text (OCR)"
Underline gesture           → "Summarize text"
Arrow gesture               → "Search related"
Cross gesture               → "Delete/hide"
Lasso gesture               → "Extract region"
Scratch gesture             → "Remove ads"
```

### AI Interpretation
- Recognizes intent, not movement
- Suggests relevant actions
- Learns from user corrections
- Context-aware suggestions

---

## Gesture Customization

Every gesture assignable.

### Customization Options
- Reassign to different actions
- Call custom extensions
- Execute macros
- Run scripts
- Trigger AI prompts
- Open specific pages
- Launch apps

### Configuration
```yaml
gestures:
  two_finger_swipe_left:
    action: "next_tab"
    alternate: "close_tab"
    mode: "normal"
    
  three_finger_tap:
    action: "custom_macro"
    macro_id: "my_workflow"
    
  force_touch_hard:
    action: "ai_assistant"
    context: "semantic"
```

---

## Device Detection

Browser automatically detects and optimizes.

### Detected Hardware
```
✓ Trackpad type (Apple, Windows Precision, libinput)
✓ Mouse presence
✓ Touchscreen
✓ Stylus/Pen
✓ Keyboard type (mechanical, membrane, laptop)
✓ Gaming keyboard (RGB, macro keys)
✓ Accessibility devices
✓ Screen resolution & DPI
✓ Refresh rate (60Hz, 120Hz, 144Hz)
```

### Adaptive UI
- Adjusts gesture sensitivity
- Shows appropriate affordances
- Enables/disables features
- Scales touch targets
- Optimizes for hardware

---

## Performance

### Latency Targets
- **Gesture latency**: < 1 frame (16ms @ 60Hz)
- **Keyboard response**: Instantaneous (< 8ms)
- **Haptic feedback**: < 5ms
- **Gesture recognition**: < 50ms
- **No dropped gestures**: 99.9% accuracy

### CPU Overhead
- Gesture processing: < 1% CPU
- Keyboard handling: < 0.5% CPU
- Gesture recognition: < 2% CPU
- Memory overhead: < 10MB

---

## Developer APIs

### Keyboard Events
```rust
pub trait KeyboardListener {
    fn on_key_down(&self, key: Key, modifiers: Modifiers);
    fn on_key_up(&self, key: Key);
    fn on_shortcut(&self, shortcut: &Shortcut);
}
```

### Gesture Events
```rust
pub trait GestureListener {
    fn on_gesture(&self, gesture: Gesture, pressure: f32);
    fn on_gesture_start(&self, gesture: Gesture);
    fn on_gesture_end(&self, gesture: Gesture, velocity: f32);
}
```

### Extension API
```rust
pub trait InputExtension {
    fn register_shortcut(&self, shortcut: Shortcut, handler: Box<dyn Fn()>);
    fn register_gesture(&self, gesture: Gesture, handler: Box<dyn Fn(GestureData)>);
    fn custom_keyboard_profile(&self, profile: KeyboardProfile);
    fn custom_gesture_mapping(&self, mapping: GestureMapping);
}
```

---

## AI Enhancements

### Local Learning (With User Consent)
- Learns shortcut usage patterns
- Suggests optimizations
- Auto-creates macros from repetition
- Context-aware command ordering
- Personalized shortcut recommendations

### Features
```
✓ Usage pattern analysis
✓ Frequently-used gesture optimization
✓ Macro creation from repetitive workflows
✓ Context-aware command palette
✓ Cross-device sync (optional)
✓ Privacy-first (all local)
```

### User Control
- Opt-in learning
- View learning data
- Delete learning data
- Export patterns
- Pause learning anytime

---

## Implementation Timeline

### Phase 6a (Weeks 1-4): Foundation
- [ ] Keyboard event handling framework
- [ ] Shortcut registration system
- [ ] Settings UI for customization
- [ ] Default shortcut bindings
- [ ] Command palette implementation

### Phase 6b (Weeks 5-8): Trackpad
- [ ] Gesture recognition engine
- [ ] Multi-touch support
- [ ] Haptic feedback integration
- [ ] Platform-specific trackpad drivers
- [ ] Gesture customization UI

### Phase 6c (Weeks 9-12): AI & Polish
- [ ] AI gesture interpretation
- [ ] Macro recording/playback
- [ ] Usage pattern learning
- [ ] Performance optimization
- [ ] Comprehensive testing

---

## Test Coverage

### Unit Tests
- 100+ keyboard shortcut tests
- 100+ gesture recognition tests
- 50+ macro operation tests
- 50+ AI interpretation tests

### Integration Tests
- Cross-platform shortcut behavior
- Trackpad gesture on all hardware
- Macro execution workflows
- Gesture customization apply

### Platform-Specific Tests
- Windows Precision Touchpad
- macOS Force Touch
- Linux libinput (including lightweight Linux-based laptops)

---

## Success Metrics

### Adoption
- [ ] 80% of power users use keyboard mode
- [ ] 60% of users customize shortcuts
- [ ] 40% of users create macros
- [ ] 30% of gestures use multi-touch

### Performance
- [ ] Keyboard latency < 8ms (100%)
- [ ] Gesture recognition 99.5% accuracy
- [ ] No dropped inputs (99.99%)
- [ ] CPU overhead < 2%

### Satisfaction
- [ ] User satisfaction > 4.5/5
- [ ] Accessibility score A+
- [ ] Zero keyboard-related bugs
- [ ] No performance regressions

---

## Related Documentation

- [PLATFORM_SPECIFIC.md](./PLATFORM_SPECIFIC.md) - Platform implementation details
- [INSTALLATION.md](./INSTALLATION.md) - Installation guide
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Overall architecture

---

**Status**: Specification complete, awaiting Phase 6 implementation  
**Next**: Create input module framework, platform-specific drivers
