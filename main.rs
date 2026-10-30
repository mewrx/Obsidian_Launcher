use chrono::{DateTime, Local};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

struct Config {
    vault_path: String,
    daily_folder: String,
    daily_format: String,
    daily_template: String,
    insert_position: String,
    insert_timestamp: bool,
    list_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/eno".into());
        Config {
            vault_path: format!("{home}/Documents/Obs"),
            daily_folder: "Journal".into(),
            daily_format: "%Y-%m-%d".into(),
            daily_template: "templates/+Daily note.md".into(),
            insert_position: "top".into(),
            insert_timestamp: true,
            list_limit: 50,
        }
    }
}

fn default_toml_content() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/eno".into());
    format!(
        "# noctalia-obsidian config\n\
vault_path = \"{home}/Documents/Obs\"\n\
daily_folder = \"Journal\"                       # relative to vault\n\
daily_format = \"%Y-%m-%d\"                      # strftime, note file name without .md\n\
daily_template = \"templates/+Daily note.md\"    # relative to vault\n\
insert_position = \"top\"                        # \"top\" (under frontmatter/heading) | \"bottom\"\n\
insert_timestamp = true                        # prefix entries with HH:MM\n\
list_limit = 50                                # max results for /obs\n"
    )
}

fn config_path() -> PathBuf {
    if let Some(p) = std::env::var_os("NOCTALIA_OBSIDIAN_CONFIG") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("noctalia-obsidian/config.toml")
}

fn load_config() -> Result<Config, String> {
    let p = config_path();
    if !p.exists() {
        if let Some(dir) = p.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("create config dir: {e}"))?;
        }
        fs::write(&p, default_toml_content()).map_err(|e| format!("write default config: {e}"))?;
    }
    let s = fs::read_to_string(&p).map_err(|e| format!("read config: {e}"))?;
    Ok(parse_config(&s))
}

// Minimal parser for flat `key = value` TOML (strings, bools, integers, # comments).
fn parse_config(src: &str) -> Config {
    let mut cfg = Config::default();
    for line in src.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        let v = strip_comment(v.trim());
        let sv = v.trim();
        let string = || -> String {
            let t = sv.trim_matches('"');
            t.replace("\\\"", "\"").replace("\\\\", "\\")
        };
        match k {
            "vault_path" => cfg.vault_path = string(),
            "daily_folder" => cfg.daily_folder = string(),
            "daily_format" => cfg.daily_format = string(),
            "daily_template" => cfg.daily_template = string(),
            "insert_position" => cfg.insert_position = string(),
            "insert_timestamp" => cfg.insert_timestamp = sv == "true",
            "list_limit" => {
                if let Ok(n) = sv.parse() {
                    cfg.list_limit = n
                }
            }
            _ => {}
        }
    }
    cfg
}

fn strip_comment(v: &str) -> &str {
    let mut in_str = false;
    for (i, c) in v.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &v[..i],
            _ => {}
        }
    }
    v
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn system_locale() -> chrono::Locale {
    let raw = std::env::var("LC_TIME")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let code = raw.split('.').next().unwrap_or("").trim();
    chrono::Locale::try_from(code).unwrap_or(chrono::Locale::POSIX)
}

fn fmt_time(now: &DateTime<Local>, f: &str) -> String {
    let mut s = String::new();
    let loc = system_locale();
    if write!(s, "{}", now.format_localized(f, loc)).is_err() {
        return f.to_string();
    }
    s
}

// Obsidian core templates use moment.js tokens; map the common ones to strftime.
fn moment_to_strftime(f: &str) -> String {
    let pairs = [
        ("YYYY", "%Y"), ("YY", "%y"),
        ("MMMM", "%B"), ("MMM", "%b"), ("MM", "%m"), ("M", "%-m"),
        ("DD", "%d"), ("D", "%-d"),
        ("dddd", "%A"), ("ddd", "%a"),
        ("HH", "%H"), ("H", "%-H"),
        ("hh", "%I"), ("h", "%-I"),
        ("mm", "%M"), ("m", "%-M"),
        ("ss", "%S"), ("s", "%-S"),
        ("A", "%p"), ("a", "%P"),
    ];
    let mut out = String::new();
    let mut s = f;
    while !s.is_empty() {
        let mut matched = false;
        for (token, strf) in &pairs {
            if s.starts_with(token) {
                out.push_str(strf);
                s = &s[token.len()..];
                matched = true;
                break;
            }
        }
        if !matched {
            let mut chars = s.chars();
            let c = chars.next().unwrap();
            out.push(c);
            s = chars.as_str();
        }
    }
    out
}

fn expand_tag(tag: &str, now: &DateTime<Local>, title: &str) -> Option<String> {
    let (name, fmt) = match tag.split_once(':') {
        Some((n, f)) => (n.trim(), Some(f.trim())),
        None => (tag.trim(), None),
    };
    match name {
        "title" => Some(title.to_string()),
        "date" => Some(fmt_time(now, &fmt.map(moment_to_strftime).unwrap_or("%Y-%m-%d".into()))),
        "time" => Some(fmt_time(now, &fmt.map(moment_to_strftime).unwrap_or("%H:%M".into()))),
        _ => None,
    }
}

fn render_template(tpl: &str, now: &DateTime<Local>, title: &str) -> String {
    let mut out = String::new();
    let mut rest = tpl;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                match expand_tag(&after[..end], now, title) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push_str("{{");
                        out.push_str(&after[..end]);
                        out.push_str("}}");
                    }
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn daily_path(cfg: &Config, now: &DateTime<Local>) -> PathBuf {
    Path::new(&cfg.vault_path)
        .join(&cfg.daily_folder)
        .join(format!("{}.md", fmt_time(now, &cfg.daily_format)))
}

fn ensure_note(cfg: &Config, now: &DateTime<Local>) -> Result<PathBuf, String> {
    let p = daily_path(cfg, now);
    if p.exists() {
        return Ok(p);
    }
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("create dir: {e}"))?;
    }
    let mut tpl_path = Path::new(&cfg.vault_path).join(&cfg.daily_template);
    if !tpl_path.exists() {
        let with_ext = PathBuf::from(format!("{}.md", tpl_path.display()));
        if with_ext.exists() {
            tpl_path = with_ext;
        }
    }
    let tpl = fs::read_to_string(&tpl_path).unwrap_or_default();
    if tpl.contains("<%") {
        eprintln!("warning: Templater syntax (<% %>) is not supported, left as is");
    }
    let title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let content = render_template(&tpl, now, &title);
    match OpenOptions::new().write(true).create_new(true).open(&p) {
        Ok(mut f) => f.write_all(content.as_bytes()).map_err(|e| format!("write note: {e}"))?,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("create note: {e}")),
    }
    Ok(p)
}

fn insert_entry(content: &str, entry: &str, top: bool) -> String {
    let nl = if content.contains("\r\n") { "\r\n" } else { "\n" };
    if !top {
        let mut s = content.to_string();
        if !s.is_empty() && !s.ends_with('\n') {
            s.push_str(nl);
        }
        s.push_str(entry);
        s.push_str(nl);
        return s;
    }
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let mut idx = 0;
    if lines.first().map(|l| l.trim_end() == "---").unwrap_or(false) {
        if let Some(p) = lines.iter().skip(1).position(|l| l.trim_end() == "---") {
            idx = p + 2;
        }
    }
    // first non-blank line after frontmatter: if it is a heading, insert right below it
    if let Some(h) = (idx..lines.len()).find(|&i| !lines[i].trim().is_empty()) {
        if lines[h].trim_start().starts_with('#') {
            idx = h + 1;
        }
    }
    let mut out: String = lines[..idx].concat();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(entry);
    out.push_str(nl);
    out.push_str(&lines[idx..].concat());
    out
}

fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| format!("write tmp: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename: {e}"))
}

fn url_encode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => {
                let _ = write!(o, "%{:02X}", b);
            }
        }
    }
    o
}

fn open_in_obsidian(path: &Path) -> Result<(), String> {
    let uri = format!("obsidian://open?path={}", url_encode(&path.to_string_lossy()));
    Command::new("xdg-open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("xdg-open: {e}"))
}

fn notify(msg: &str) {
    let _ = Command::new("notify-send").args(["-a", "Obsidian", "Obsidian", msg]).spawn();
}

fn list_notes(cfg: &Config, query: &str) -> Vec<(String, u64)> {
    let root = PathBuf::from(&cfg.vault_path);
    let tpl_dir = Path::new(&cfg.daily_template).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut stack = vec![root.clone()];
    let mut notes: Vec<(String, u64)> = vec![];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            let Ok(ft) = e.file_type() else { continue };
            let rel = path.strip_prefix(&root).unwrap_or(&path).to_path_buf();
            if ft.is_dir() {
                if !tpl_dir.as_os_str().is_empty() && rel == tpl_dir {
                    continue;
                }
                stack.push(path);
            } else if name.ends_with(".md") {
                let mtime = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                notes.push((rel.to_string_lossy().to_string(), mtime));
            }
        }
    }
    let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    notes.retain(|(rel, _)| {
        let l = rel.to_lowercase();
        tokens.iter().all(|t| l.contains(t))
    });
    notes.sort_by(|a, b| b.1.cmp(&a.1));
    notes.truncate(cfg.list_limit);
    notes
}

fn run(args: &[String]) -> Result<(), String> {
    let cfg = load_config()?;
    match args.first().map(|s| s.as_str()) {
        Some("day") => {
            let text = args[1..].join(" ").replace(['\n', '\r'], " ");
            let text = text.trim();
            let now = Local::now();
            let note = ensure_note(&cfg, &now)?;
            if text.is_empty() {
                return open_in_obsidian(&note);
            }
            let content = fs::read_to_string(&note).map_err(|e| format!("read note: {e}"))?;
            let entry = if cfg.insert_timestamp {
                format!("- {} {}", now.format("%H:%M"), text)
            } else {
                format!("- {}", text)
            };
            let updated = insert_entry(&content, &entry, cfg.insert_position != "bottom");
            write_atomic(&note, &updated)?;
            notify(&format!("Added to {}", note.file_name().unwrap_or_default().to_string_lossy()));
            Ok(())
        }
        Some("obs") => {
            let root = PathBuf::from(&cfg.vault_path);
            for (rel, _) in list_notes(&cfg, &args[1..].join(" ")) {
                let p = Path::new(&rel);
                let title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
                println!(
                    "{{\"title\":{},\"dir\":{},\"path\":{}}}",
                    json_str(&title),
                    json_str(&dir),
                    json_str(&root.join(&rel).to_string_lossy())
                );
            }
            Ok(())
        }
        Some("open") => {
            let p = args.get(1).ok_or("open: missing path")?;
            open_in_obsidian(Path::new(p))
        }
        _ => Err("usage: noctalia-obsidian day [text] | obs [query] | open <path>".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("{e}");
        notify(&e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_with_frontmatter_and_heading() {
        let c = "---\ntags: daily\n---\n\n# Notes\n\nold\n";
        let r = insert_entry(c, "- 10:00 new", true);
        assert_eq!(r, "---\ntags: daily\n---\n\n# Notes\n- 10:00 new\n\nold\n");
    }

    #[test]
    fn top_with_frontmatter_no_heading() {
        let c = "---\na: b\n---\ntext\n";
        assert_eq!(insert_entry(c, "- x", true), "---\na: b\n---\n- x\ntext\n");
    }

    #[test]
    fn top_without_frontmatter() {
        assert_eq!(insert_entry("body\n", "- x", true), "- x\nbody\n");
        assert_eq!(insert_entry("", "- x", true), "- x\n");
    }

    #[test]
    fn top_heading_is_last_line_without_newline() {
        assert_eq!(insert_entry("# H", "- x", true), "# H\n- x\n");
    }

    #[test]
    fn bottom_appends() {
        assert_eq!(insert_entry("a\nb", "- x", false), "a\nb\n- x\n");
        assert_eq!(insert_entry("a\n", "- x", false), "a\n- x\n");
    }

    #[test]
    fn crlf_preserved() {
        let r = insert_entry("---\r\na: b\r\n---\r\n# H\r\n", "- x", true);
        assert_eq!(r, "---\r\na: b\r\n---\r\n# H\r\n- x\r\n");
    }

    #[test]
    fn template_rendering() {
        let now = Local::now();
        let r = render_template("# {{title}}\n{{date}} {{date:YYYY/MM/DD}} {{unknown}}", &now, "T");
        println!("render_template result: {:?}", r);
        println!("expected: {:?}", now.format("%Y/%m/%d").to_string());
        assert!(r.starts_with("# T\n"));
        assert!(r.contains(&now.format("%Y-%m-%d").to_string()));
        assert!(r.contains(&now.format("%Y/%m/%d").to_string()));
        assert!(r.ends_with("{{unknown}}"));
    }

    #[test]
    fn config_parsing() {
        let c = parse_config("vault_path = \"/x/y\"  # c\ninsert_position = \"bottom\"\ninsert_timestamp = false\nlist_limit = 7\ndaily_template = \"a#b.md\"\n");
        assert_eq!(c.vault_path, "/x/y");
        assert_eq!(c.insert_position, "bottom");
        assert!(!c.insert_timestamp);
        assert_eq!(c.list_limit, 7);
        assert_eq!(c.daily_template, "a#b.md");
        assert_eq!(c.daily_folder, "Journal");
    }

    #[test]
    fn json_escape() {
        assert_eq!(json_str("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn encode() {
        assert_eq!(url_encode("/a b/+c.md"), "%2Fa%20b%2F%2Bc.md");
    }

    #[test]
    fn test_ukrainian_date() {
        use chrono::Locale;
        let loc = Locale::try_from("uk_UA").unwrap_or(Locale::POSIX);
        let now = Local::now();
        let s = format!("{}", now.format_localized("%A, %-d %B %Y", loc));
        println!("Ukrainian date: {}", s);
        assert!(s.contains("жовтня"));

        let rendered = render_template("# {{date:dddd, D MMMM YYYY}}", &now, "test");
        println!("rendered header: {}", rendered);
        assert!(rendered.starts_with("# "));
        assert!(rendered.contains("жовтня"));
    }
}
