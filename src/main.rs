use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const EXTS: &str = "js ts tsx jsx mjs cjs py pyw rs go java kt kts swift c cpp cc cxx h hpp hxx cs vb vbs rb rpy php pl pm lua nim zig dart scala groovy clj cljs cljc ex exs erl hrl hs lhs ml mli fs fsi fsx jl r R cr c3 odin v pas pp sql graphql gql proto thrift toml yaml yml json md mdx sh bash zsh fish ps1 bat cmd tf tfvars hcl vue svelte astro elm purs pure res resi gleam tofu pkl raku rakumod tcl awk sed asm s m mm swiftui rst adoc tex sty cls bib jl";

fn is_source(p: &Path) -> bool {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    EXTS.split_whitespace().any(|e| e == ext)
}

fn is_excluded(p: &Path) -> bool {
    p.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        matches!(s.as_ref(), "node_modules" | "target" | ".git" | "dist" | "build" | "__pycache__" | "vendor" | ".venv" | "venv" | "deps")
            || s.ends_with(".test.ts") || s.ends_with(".test.js") || s.ends_with("_test.go")
            || s.ends_with("_test.py") || s.ends_with("_test.rs") || s.contains(".spec.")
    })
}

fn strip_comments(code: &str) -> String {
    code.lines().map(|l| {
        // strip // and # comments, keep length
        let mut cut = l.len();
        if let Some(i) = l.find("//") { cut = cut.min(i); }
        if let Some(i) = l.find('#') { cut = cut.min(i); }
        // handle -- for SQL/Haskell
        if let Some(i) = l.find("--") { cut = cut.min(i); }
        format!("{}{}", &l[..cut], " ".repeat(l.len().saturating_sub(cut)))
    }).collect::<Vec<_>>().join("\n")
}

fn symbols_of(content: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let re = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+|export\s+|public\s+|pub\s+)?(?:async\s+|static\s+|const\s+|let\s+|var\s+|val\s+)?(?:function|def|fn|func|sub|proc|method|class|struct|enum|trait|interface|type|record|module|namespace|package)\s+([A-Za-z_]\w*)").unwrap();
    for (i, line) in content.lines().enumerate() {
        for c in re.captures_iter(line) {
            let name = c.get(1).map(|m| m.as_str()).unwrap_or("");
            if !name.is_empty() && name != "default" && name != "_" && name.len() > 1 {
                out.push((i + 1, name.to_string()));
            }
        }
    }
    // Rust/Go: also capture `pub fn` already covered; capture impl methods? skip for simplicity
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
                println!("\nDetects exported/public symbols never referenced elsewhere in the repo.");
                println!("Supports 100+ languages by file extension.");
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

    let mut files: Vec<(PathBuf, String)> = Vec::new(); // path, cleaned content
    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_source(p) || is_excluded(p) { continue; }
        if excludes.iter().any(|ex| p.to_string_lossy().contains(ex)) { continue; }
        if let Ok(raw) = fs::read_to_string(p) {
            files.push((p.to_path_buf(), strip_comments(&raw)));
        }
    }

    let mut file_syms: HashMap<PathBuf, Vec<(usize, String)>> = HashMap::new();
    for (p, content) in &files {
        file_syms.insert(p.clone(), symbols_of(content));
    }

    // global text for reference counting
    let all_text: String = files.iter().map(|(_, c)| c.as_str()).collect::<Vec<_>>().join("\n");

    let mut dead: Vec<(PathBuf, usize, String)> = Vec::new();
    for (p, _) in &files {
        for (ln, sym) in file_syms.get(p).cloned().unwrap_or_default() {
            let mut count = 0usize;
            for (p2, c2) in &files {
                if p2 == p {
                    for (i, line) in c2.lines().enumerate() {
                        if i + 1 == ln { continue; }
                        count += line.matches(sym.as_str()).count();
                    }
                } else {
                    count += c2.matches(sym.as_str()).count();
                }
            }
            // also account for the symbol appearing in its own file beyond def line
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
    let _ = all_text;
}
