use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Copy, PartialEq)]
enum Lang { Js, Ts, Py, Rs, Go, Unknown }

fn lang_of(p: &Path) -> Lang {
    match p.extension().and_then(|e| e.to_str()) {
        Some("js" | "jsx") => Lang::Js,
        Some("ts" | "tsx") => Lang::Ts,
        Some("py") => Lang::Py,
        Some("rs") => Lang::Rs,
        Some("go") => Lang::Go,
        _ => Lang::Unknown,
    }
}

fn is_excluded(p: &Path) -> bool {
    p.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        matches!(s.as_ref(), "node_modules" | "target" | ".git" | "dist" | "build" | "__pycache__" | "vendor" | ".venv" | "venv")
            || s.ends_with(".test.ts") || s.ends_with(".test.js") || s.ends_with("_test.go") || s.ends_with("_test.py")
            || s.contains(".spec.")
    })
}

fn is_source(p: &Path) -> bool { !matches!(lang_of(p), Lang::Unknown) }

fn strip_comments(code: &str, lang: Lang) -> String {
    // Remove line comments for the language; keep newlines for line numbers.
    let line = match lang {
        Lang::Py => "#",
        Lang::Rs | Lang::Go | Lang::Js | Lang::Ts => "//",
        _ => "//",
    };
    code.lines().map(|l| {
        if let Some(i) = l.find(line) { format!("{}{}", &l[..i], " ".repeat(l.len().saturating_sub(i))) } else { l.to_string() }
    }).collect::<Vec<_>>().join("\n")
}

fn exports_of(content: &str, lang: Lang) -> Vec<(usize, String)> {
    // returns (line_number, symbol)
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let push = |out: &mut Vec<(usize, String)>, ln: usize, name: &str| {
        if !name.is_empty() && name != "default" && name != "_" {
            out.push((ln, name.to_string()));
        }
    };

    match lang {
        Lang::Js | Lang::Ts => {
            let re = Regex::new(r"export\s+(?:default\s+)?(?:async\s+)?(?:function|const|let|var|class|interface|type|enum)\s+([A-Za-z_]\w*)").unwrap();
            for (i, line) in lines.iter().enumerate() {
                for c in re.captures_iter(line) { push(&mut out, i + 1, &c[1]); }
            }
            let re2 = Regex::new(r"export\s*\{\s*([^}]+?)\s*\}").unwrap();
            for (i, line) in lines.iter().enumerate() {
                for c in re2.captures_iter(line) {
                    for part in c[1].split(',') {
                        let name = part.trim().split_whitespace().next().unwrap_or("").trim_matches('"').trim_matches('\'');
                        push(&mut out, i + 1, name);
                    }
                }
            }
        }
        Lang::Py => {
            let re = Regex::new(r"^(?:async\s+)?def\s+([A-Za-z_]\w*)\s*\(|^class\s+([A-Za-z_]\w*)").unwrap();
            for (i, line) in lines.iter().enumerate() {
                for c in re.captures_iter(line) {
                    if let Some(m) = c.get(1).or(c.get(2)) { push(&mut out, i + 1, m.as_str()); }
                }
            }
        }
        Lang::Rs => {
            let re = Regex::new(r"pub\s+(?:async\s+)?(?:fn|struct|enum|trait|const|static|type|mod)\s+([A-Za-z_]\w*)").unwrap();
            for (i, line) in lines.iter().enumerate() {
                for c in re.captures_iter(line) { push(&mut out, i + 1, &c[1]); }
            }
        }
        Lang::Go => {
            let re = Regex::new(r"^func\s+(?:\([^)]*\)\s+)?([A-Z]\w*)").unwrap();
            for (i, line) in lines.iter().enumerate() {
                for c in re.captures_iter(line) { push(&mut out, i + 1, &c[1]); }
            }
        }
        Lang::Unknown => {}
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut root = ".".to_string();
    let mut json = false;
    let mut excludes: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--exclude" => { i += 1; if i < args.len() { excludes.push(args[i].clone()); } }
            "-h" | "--help" => {
                println!("deadcode - fast cross-language dead-code detector\n");
                println!("Usage: deadcode [PATH] [--json] [--exclude PATTERN]");
                println!("\nDetects exported symbols never referenced anywhere else in the repo.");
                println!("Supports: JS, TS, TSX, JSX, Python, Rust, Go.");
                println!("\nOptions:");
                println!("  --json            Output JSON");
                println!("  --exclude PAT     Skip paths containing PAT (repeatable)");
                println!("  -h, --help        Show this help");
                return;
            }
            other => { if !other.starts_with('-') { root = other.to_string(); } }
        }
        i += 1;
    }

    // collect files
    let mut files: Vec<(PathBuf, Lang, String)> = Vec::new();
    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_source(p) || is_excluded(p) { continue; }
        if excludes.iter().any(|ex| p.to_string_lossy().contains(ex)) { continue; }
        let lang = lang_of(p);
        if let Ok(raw) = fs::read_to_string(p) {
            files.push((p.to_path_buf(), lang, strip_comments(&raw, lang)));
        }
    }

    // exports per file
    let mut file_exports: HashMap<PathBuf, Vec<(usize, String)>> = HashMap::new();
    for (p, lang, content) in &files {
        file_exports.insert(p.clone(), exports_of(content, *lang));
    }

    // build a global "reference count" excluding the defining file's export line
    // A symbol is considered used if it appears in any file other than its definer,
    // OR appears more than once in its definer (re-export / self-reference counts as use).
    let mut all_text = String::new();
    for (_, _, c) in &files { all_text.push_str(c); all_text.push('\n'); }

    let mut dead: Vec<(PathBuf, usize, String)> = Vec::new();
    for (p, _, _) in &files {
        for (ln, sym) in file_exports.get(p).cloned().unwrap_or_default() {
            // count occurrences across all files
            let mut count = 0;
            for (p2, _, c2) in &files {
                // in defining file, count only non-export occurrences
                if p2 == p {
                    for (i, line) in c2.lines().enumerate() {
                        if i + 1 == ln { continue; } // skip the export line itself
                        count += line.matches(&sym).count();
                    }
                } else {
                    count += c2.matches(&sym).count();
                }
            }
            if count == 0 {
                dead.push((p.clone(), ln, sym));
            }
        }
    }

    if json {
        println!("{{\"dead\":{},\"files\":{}}}", dead.len(), files.len());
        for (p, ln, sym) in &dead {
            println!("{{\"file\":\"{}\",\"line\":{},\"symbol\":\"{}\"}}", p.display(), ln, sym);
        }
    } else {
        for (p, ln, sym) in &dead {
            println!("DEAD  {}:{}  {}", p.display(), ln, sym);
        }
        println!("\n{} dead symbol(s) found across {} files.", dead.len(), files.len());
    }
}
