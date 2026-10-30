# Noctalia Obsidian Launcher

A fast, lightweight launcher plugin for [Noctalia Shell](https://noctalia.dev) (v5+) that integrates directly with an [Obsidian](https://obsidian.md) vault. It provides quick-capture daily journaling via `/day` and lightning-fast vault note search via `/obs`.

---

## Overview

Taking quick notes or jumping to existing thoughts should not require opening the full Obsidian window, navigating directories, or waiting for workspace reloads. 

**Noctalia Obsidian Launcher** integrates Obsidian directly into Noctalia's native Wayland launcher:
- **/day**: Creates or opens today's daily note based on your vault template, or appends a timestamped entry under today's heading without opening the editor.
- **/obs**: Searches your Obsidian vault by note title and directory name, displaying instant cached results and opening selected notes directly in Obsidian.

---

## Features

### 1. Daily Note Capture (`/day`)
- **Open Today's Note**: Typing `/day` with no arguments presents an option to open today's daily note. If the note does not exist yet, it is automatically generated from your template before opening.
- **Quick Capture**: Typing `/day <text>` immediately appends a bulleted entry with a 24-hour timestamp (`- HH:MM <text>`) to today's note.
- **Smart Heading Placement**: New entries are placed right beneath the primary heading (or below YAML frontmatter), with the newest entry placed on top (`insert_position = "top"`), or appended to the bottom if configured.
- **Template Rendering & Ukrainian / System Localization**: Generates daily notes from vault templates (e.g., `templates/+Daily note.md`). Moment.js date tokens (`YYYY`, `MMMM`, `dddd`, `D`, etc.) are expanded using your system locale (e.g., producing `# п'ятниця, 30 жовтня 2026`).
- **Atomic File Writes**: Writes modified notes atomically using a temporary file and atomic rename to prevent corruption.
- **Desktop Notification**: Emits a desktop notification via `notify-send` confirming capture.

### 2. Note Search (`/obs`)
- **Instant Search**: Notes are indexed and cached in memory for sub-millisecond filtering as you type.
- **Background Refresh**: The Rust helper asynchronously scans the vault in the background (typically taking < 10 ms for 1,500+ notes) and refreshes the launcher view with newest-first ordering.
- **Deep Direct Opening**: Selecting any result opens that note in Obsidian via `obsidian://open?path=...`.

---

## Architecture

The extension consists of two cooperating layers:

```
┌────────────────────────────────────────────────────────┐
│                   Noctalia Shell (v5+)                 │
│                                                        │
│   Launcher UI  <──>  Luau Plugin (day / obs)          │
│                      - Restrictive Luau sandbox        │
│                      - Instant synchronous cache       │
│                      - Exact query string matching     │
└──────────────────────────┬─────────────────────────────┘
                           │ noctalia.runAsync()
                           ▼
┌────────────────────────────────────────────────────────┐
│            Rust Helper: noctalia-obsidian              │
│            (~/.local/bin/noctalia-obsidian)            │
│                                                        │
│   - Reads ~/.config/noctalia-obsidian/config.toml      │
│   - Expands templates & localizes dates                │
│   - Atomic note updates & frontmatter parsing          │
│   - Fast recursive vault scanner                       │
│   - Dispatches xdg-open & notify-send                  │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│                   Obsidian Vault                       │
│           (e.g., ~/Documents/Obs)                      │
│   - Journal/YYYY-MM-DD.md                              │
│   - templates/+Daily note.md                           │
└────────────────────────────────────────────────────────┘
```

---

## Requirements and Dependencies

1. **Noctalia Shell** v5.0+ (the current native C++/Rust Wayland shell with Luau plugins).
2. **Rust & Cargo** (1.70+ recommended to build the helper binary).
3. **Obsidian** (desktop app installed and registered with `xdg-open` for `obsidian://` URIs).
4. **Core Utilities**: `xdg-open`, `notify-send` (libnotify).

---

## Installation

### Automated Installation

Clone the repository and run the provided non-destructive installer:

```bash
git clone https://github.com/<your-username>/Obsidian_Launcher.git
cd Obsidian_Launcher
chmod +x install.sh
./install.sh
```

The script will:
1. Compile `noctalia-obsidian` in `--release` mode.
2. Install the binary to `~/.local/bin/noctalia-obsidian`.
3. Create default configuration at `~/.config/noctalia-obsidian/config.toml` if it does not already exist.
4. Deploy the plugin files (`plugin.toml`, `day_launcher.luau`, `obs_launcher.luau`) to `~/.config/noctalia/plugins/obsidian/`.
5. Validate the plugin with `noctalia plugins lint` and reload Noctalia.

---

### Manual Installation

If you prefer to install manually:

1. **Build and install the Rust helper**:
   ```bash
   cargo build --release
   mkdir -p ~/.local/bin
   cp target/release/noctalia-obsidian ~/.local/bin/
   chmod +x ~/.local/bin/noctalia-obsidian
   ```

2. **Initialize configuration**:
   Run the helper once to generate default configuration:
   ```bash
   ~/.local/bin/noctalia-obsidian --help || true
   ```
   Edit `~/.config/noctalia-obsidian/config.toml` to match your Obsidian vault paths.

3. **Deploy the Noctalia plugin**:
   ```bash
   mkdir -p ~/.config/noctalia/plugins/obsidian
   cp plugin.toml day_launcher.luau obs_launcher.luau ~/.config/noctalia/plugins/obsidian/
   ```

4. **Enable and reload**:
   ```bash
   noctalia plugins lint ~/.config/noctalia/plugins/obsidian
   noctalia msg plugins enable local/obsidian
   noctalia msg config-reload
   ```

---

## Configuration

Configuration is stored in `~/.config/noctalia-obsidian/config.toml`:

```toml
# noctalia-obsidian configuration
vault_path = "/home/eno/Documents/Obs"           # Absolute path to your vault
daily_folder = "Journal"                         # Directory inside vault for daily notes
daily_format = "%Y-%m-%d"                        # File name format (strftime, without .md)
daily_template = "templates/+Daily note.md"      # Relative path to template file
insert_position = "top"                          # "top" (under heading) or "bottom"
insert_timestamp = true                          # Prefix entries with HH:MM
list_limit = 50                                  # Maximum results returned for /obs
```

Override configuration path during development using `NOCTALIA_OBSIDIAN_CONFIG=/path/to/config.toml`.

---

## Usage

Open the Noctalia launcher (default: `Mod+Space`).

### 1. Today's Note (`/day`)
- Type `/day`:
  - Result: `Open today's daily note`
  - Action: Press `Enter` to create/open today's note in Obsidian.
- Type `/day Buy groceries and check server`:
  - Result: `Add to daily note: Buy groceries and check server`
  - Action: Press `Enter` to append `- 18:45 Buy groceries and check server` right under the heading in today's daily note.

### 2. Vault Search (`/obs`)
- Type `/obs`:
  - Displays the 50 most recently modified notes in the vault.
- Type `/obs project`:
  - Instantly filters notes matching `project` in either the title or directory name.
  - Press `Enter` on any note to open it in Obsidian.

---

## Troubleshooting & Debugging Insights

### 1. `os.getenv()` is unavailable in the Luau sandbox
- **Symptom**: The launcher loads the buttons, but clicking them or typing produces no results.
- **Cause**: Noctalia runs plugins in a secure Luau environment where the standard Lua `os.getenv` function is `nil`. Calling it causes an immediate runtime crash when the script loads.
- **Solution**: Use `noctalia.expandPath("~/.local/bin/noctalia-obsidian")` or `noctalia.getenv("HOME")`.

### 2. Stale query mismatch in `launcher.setResults()`
- **Symptom**: Typing produces no visible search results.
- **Cause**: Noctalia's C++ launcher checks that the `query` argument passed into `launcher.setResults(query, results)` strictly matches the current user input buffer. If a script trims the string (e.g. trimming `" "` into `""`), Noctalia drops the results as belonging to an expired query.
- **Solution**: Always pass the exact, unmodified `text` string received by `onQuery(text)` as the first argument to `launcher.setResults`.

### 3. Synchronous frame requirements
- **Symptom**: Launcher results flicker or fail to render while typing.
- **Cause**: Noctalia queries launcher providers synchronously on every keystroke.
- **Solution**: Maintain a memory cache in Luau. `onQuery` immediately calls `launcher.setResults` with filtered cached items, and then fires a background `runAsync` to refresh newer items.

### 4. Shell argument quoting
- **Symptom**: Entries with spaces, quotes, or special characters fail to append or truncate.
- **Solution**: Wrap arguments using single-quote escaping:
  ```lua
  local function shellQuote(v)
    return "'" .. tostring(v):gsub("'", "'\\''") .. "'"
  end
  ```

---

## Project Structure

```
.
├── Cargo.toml                  # Rust package manifest (chrono + unstable-locales)
├── Cargo.lock                  # Pinned dependency lockfile
├── main.rs                     # Rust helper source code
├── plugin.toml                 # Noctalia v5 plugin manifest (launcher_provider declarations)
├── day_launcher.luau           # /day Luau entry script
├── obs_launcher.luau           # /obs Luau entry script
├── install.sh                  # Non-destructive deployment script
├── .gitignore                  # Git ignore rules for build artifacts
├── README.md                   # Project documentation
├── obsidian_noctalia_launcher.md # Technical maintenance note for Obsidian
└── legacy-v4-qml/              # Archived legacy Noctalia Shell v4 (Quickshell) files
    ├── LauncherProvider.qml
    └── manifest.json
```

---

## Development & Maintenance

### Running Unit Tests
```bash
cargo test
```
The test suite covers:
- Frontmatter and heading detection.
- Insertion positions (`top` vs `bottom`).
- CRLF and newline preservation.
- Moment token parser (avoiding token collision bugs).
- Localized Ukrainian date formatting.
- Flat TOML configuration parsing.

### Linting the Noctalia Plugin
```bash
noctalia plugins lint .
```

### Hot Reloading
- Changes to `.luau` files are automatically hot-reloaded by Noctalia on save.
- If you edit `plugin.toml` or rebuild the Rust binary, run:
  ```bash
  noctalia msg config-reload
  ```

---

## Known Limitations

- **Templater Syntax**: Templater dynamic JavaScript tags (`<% ... %>`) are left intact; only standard Moment.js tags (`{{date}}`, `{{time}}`, `{{title}}`, `{{date:FORMAT}}`) are expanded by the helper.
- **Single-Line Captures**: Entries captured via `/day` are single-line bullets (`- HH:MM <text>`).
- **Wayland / xdg-open**: Relies on system `xdg-open` having a registered handler for `obsidian://` protocol.
