---
tags:
  - noctalia
  - obsidian
  - linux
  - configuration
related: "[[Obsidian tags]]"
created: 2026-10-30
---

# Noctalia Launcher Extension for Obsidian — Technical Specification & Maintenance Guide

## 1. Executive Summary & Purpose

This document is the authoritative technical reference for the **Noctalia Obsidian Launcher** extension. It is intended for system maintenance, disaster recovery, and future AI coding agents working on this codebase.

The extension integrates an Obsidian vault directly into the [Noctalia Shell](https://noctalia.dev) (v5+) application launcher under Wayland. It provides two commands:
1. `/day [text]` — Quick-capture thoughts into today's daily note under the header with a timestamp (`- HH:MM text`), or create/open today's note if no text is given.
2. `/obs [query]` — Instant vault search with local caching and direct opening via `obsidian://open?path=...`.

---

## 2. Verified Runtime Paths & Installation Layout

The working installation is distributed across four locations on the system:

| Component | Repository Path | Active Runtime Path | Role |
| :--- | :--- | :--- | :--- |
| **Rust Helper Source** | `main.rs`, `Cargo.toml` | `/home/eno/Projects/Obsidian_Launcher/` | Rust source code for compilation |
| **Rust Helper Binary** | `target/release/noctalia-obsidian` | `/home/eno/.local/bin/noctalia-obsidian` | Executable invoked by Noctalia Luau scripts |
| **Helper Config** | *(Generated on first run)* | `/home/eno/.config/noctalia-obsidian/config.toml` | Vault paths, template, format, position settings |
| **Plugin Manifest** | `plugin.toml` | `/home/eno/.config/noctalia/plugins/obsidian/plugin.toml` | Noctalia v5 plugin definition & launcher declarations |
| **Day Launcher Script** | `day_launcher.luau` | `/home/eno/.config/noctalia/plugins/obsidian/day_launcher.luau` | Luau handler for `/day` prefix |
| **Obs Launcher Script** | `obs_launcher.luau` | `/home/eno/.config/noctalia/plugins/obsidian/obs_launcher.luau` | Luau handler for `/obs` prefix |
| **Obsidian Vault** | N/A | `/home/eno/Documents/Obs` | Target vault containing `Journal/` & `templates/` |
| **Daily Note Template**| N/A | `/home/eno/Documents/Obs/templates/+Daily note.md` | Template used for generating new daily notes |

---

## 3. Architecture & Execution Flow

```
+-----------------------------------------------------------------------+
| Noctalia Shell (C++/Rust daemon, PID 1104, v5.3.0)                    |
|                                                                       |
|   LauncherPanel (Wayland overlay activated by Mod+Space)              |
|        │                                                              |
|        ├── User types '/day <text>' ──► day_launcher.luau:onQuery     |
|        │                                     │                        |
|        │                                     ├──► launcher.setResults |
|        │                                                              |
|        └── User activates entry ─────► day_launcher.luau:onActivate   |
|                                              │                        |
|                                              └──► noctalia.runAsync   |
+──────────────────────────────────────────────┼────────────────────────+
                                               │ (Fork & exec)
                                               ▼
+-----------------------------------------------------------------------+
| Helper Executable: ~/.local/bin/noctalia-obsidian                     |
|                                                                       |
|   Arguments:                                                          |
|     - day [text]                                                      |
|     - obs [query]                                                     |
|     - open <path>                                                     |
|                                                                       |
|   Actions:                                                            |
|     1. Reads ~/.config/noctalia-obsidian/config.toml                  |
|     2. If today's note missing -> renders template with system locale |
|     3. Parses frontmatter (---) & main heading (# Header)             |
|     4. Inserts entry beneath heading (atomic tempfile + rename)       |
|     5. Dispatches desktop notification (notify-send)                  |
|     6. Launches note in Obsidian via xdg-open obsidian://open?path=.. |
+───────────────────────────────────────────────────────────────────────+
```

### Detailed Flow: `/day`
1. **Trigger**: User opens launcher (`Mod+Space`) and types `/day` or clicks the `/day` chip.
2. **`onQuery(text)`**: Noctalia calls `day_launcher.luau:onQuery(text)`.
   - If `text` is empty (or whitespace): returns 1 item: `Open today's daily note`.
   - If `text` has content: returns 1 item: `Add to daily note: <text>`.
   - Calls `launcher.setResults(text, results)` with the unmodified `text` parameter.
3. **`onActivate(id)`**:
   - `id == "open"`: executes `noctalia-obsidian day`.
   - `id == "add:<text>"`: executes `noctalia-obsidian day '<safe_quoted_text>'`.
4. **Helper Processing**:
   - Locates target: `/home/eno/Documents/Obs/Journal/YYYY-MM-DD.md`.
   - If missing: reads `/home/eno/Documents/Obs/templates/+Daily note.md`, parses tags (`{{date:dddd, D MMMM YYYY}}`), formats date using `uk_UA.UTF-8` (`# п'ятниця, 30 жовтня 2026`), writes file.
   - If text provided: formats line `- HH:MM <text>`, splits content by lines, skips YAML frontmatter (`---` to `---`), finds first heading (`# ...`), inserts line right after heading, writes via atomic `.tmp` rename.
   - Dispatches `notify-send -a Obsidian Obsidian "Added to YYYY-MM-DD.md"`.

### Detailed Flow: `/obs`
1. **Startup**: When `obs_launcher.luau` loads, it triggers an initial asynchronous scan `noctalia-obsidian obs ""` to fill `notes_cache`.
2. **`onQuery(text)`**:
   - Immediately filters `notes_cache` synchronously and calls `launcher.setResults(text, filtered)`. This guarantees zero flicker and immediate rendering.
   - Spawns background update via `noctalia-obsidian obs '<query>'`. When output completes, updates `notes_cache` and calls `launcher.setResults(text, fresh_results)`.
3. **`onActivate(id)`**:
   - Extracts note path from `open:<path>`.
   - Executes `noctalia-obsidian open '<path>'`.
   - Helper invokes `xdg-open "obsidian://open?path=<url_encoded_path>"`.

---

## 4. Critical Technical Constraints & Lessons Learned

### A. The Noctalia Luau Sandbox Restriction (`os.getenv`)
- **Verified Fact**: Noctalia v5+ runs plugin entry scripts in a sandboxed Luau environment. Standard Lua modules like `os.getenv` or `os.execute` are `nil`.
- **Failure Mode**: Any access to `os.getenv("HOME")` throws `attempt to call a nil value`, preventing the script from compiling/loading.
- **Rule**: Never use `os.getenv`. Use `noctalia.expandPath("~/.local/bin/noctalia-obsidian")` or `noctalia.getenv("HOME")`.

### B. Strict Query Matching in `launcher.setResults`
- **Verified Fact**: Noctalia's C++ launcher host compares the `query` argument passed to `launcher.setResults(query, results)` with the exact internal text buffer of the launcher.
- **Failure Mode**: If `onQuery` receives `" "` (e.g. after typing `/day `) and the script trims it to `""` before calling `launcher.setResults("", results)`, Noctalia detects a query mismatch (`"" != " "`) and discards the results as stale.
- **Rule**: Always pass the exact, untouched `query` argument received in `onQuery` directly to `launcher.setResults(query, results)`.

### C. Synchronous vs Asynchronous Results
- **Verified Fact**: Noctalia renders launcher results on the current frame. If `onQuery` only launches an asynchronous callback and returns without calling `launcher.setResults`, the list remains blank.
- **Rule**: `onQuery` must always call `launcher.setResults` synchronously first (using cached items or static actions), even if an asynchronous refresh is dispatched in parallel.

### D. Shell Argument Quoting
- **Verified Fact**: Commands passed to `noctalia.runAsync(string)` are executed through a shell interpreter.
- **Rule**: Any user-supplied text or file paths containing spaces, quotes, or special characters must be escaped using single-quote wrapping:
  ```lua
  local function shellQuote(v)
    return "'" .. tostring(v):gsub("'", "'\\''") .. "'"
  end
  ```

### E. Date Formatting & Token Collisions in Rust
- **Verified Fact**: In Obsidian templates, formats like `{{date:dddd, D MMMM YYYY}}` use Moment.js tokens where `D` means unpadded day of month.
- **Failure Mode**: Sequential string `.replace("MM", "%m").replace("m", "%-M")` corrupts earlier replacements because the replacement `%m` contains the character `m`.
- **Rule**: The token converter in `main.rs:moment_to_strftime` uses single-pass greedy token scanning to prevent recursive replacement bugs. Locale formatting uses `chrono::DateTime::format_localized` with the host's `system_locale()`.

---

## 5. Maintenance Checklist for Future AI Agents

Before making any modifications to this project, follow this protocol:

1. **Check Environment First**:
   - Run `noctalia msg status` to verify Noctalia daemon status.
   - Run `noctalia msg plugins list | grep obsidian` to check registration.
   - Run `~/.local/bin/noctalia-obsidian --help` to verify helper execution.
2. **Preserve Luau Signatures**:
   - The launcher interface requires:
     - `function onQuery(text: string)`
     - `function onActivate(id: string)`
   - Never rename these to `search` or `activate`.
3. **Validate Code Changes**:
   - Always run unit tests: `cargo test` in repository root. All tests must pass.
   - Always lint Noctalia plugin: `noctalia plugins lint ~/.config/noctalia/plugins/obsidian/`.
4. **Deploy & Reload**:
   - If `main.rs` changes: `cargo build --release && cp target/release/noctalia-obsidian ~/.local/bin/`.
   - If Luau changes: copy updated `.luau` files to `~/.config/noctalia/plugins/obsidian/`.
   - Reload Noctalia config: `noctalia msg config-reload`.
5. **Regression Verification**:
   - Test `/day`: open launcher, verify `Open today's daily note` prompt.
   - Test `/day text`: verify capture creates or updates `Journal/YYYY-MM-DD.md`.
   - Test `/obs`: verify instant list of vault notes appears.
