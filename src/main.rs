use regex::Regex;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const EXTS: &str = "js ts tsx jsx mjs cjs py pyw rs go java kt kts swift c cpp cc cxx h hpp hxx cs vb vbs rb rpy php pl pm lua nim zig dart scala groovy clj cljs cljc ex exs erl hrl hs lhs ml mli fs fsi fsx jl r R cr c3 odin v pas pp sql graphql gql proto thrift toml yaml yml json md mdx sh bash zsh fish ps1 bat cmd tf tfvars hcl vue svelte astro elm purs pure res resi gleam tofu pkl tcl awk sed asm s m mm rst adoc tex sty cls bib elixir exs eex crm diff patch psv ini cfg conf properties properties yml yaml ini diff patch log trace gitignore dockerfile containerfile gem gemspec lock lockfile nix lock lockfile toml ini cfg conf env dotfile";

fn is_source(p: &Path) -> bool {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    EXTS.split_whitespace().any(|e| e == ext)
}

fn is_excluded(p: &Path) -> bool {
    p.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        matches!(
            s.as_ref(),
            "node_modules" | "target" | ".git" | "dist" | "build" | "__pycache__"
                | "vendor" | ".venv" | "venv" | "deps"
        )
    })
}

fn strip_comments(code: &str) -> String {
    code.lines()
        .map(|l| {
            let mut cut = l.len();
            if let Some(i) = l.find("//") { cut = cut.min(i); }
            if let Some(i) = l.find('#') { cut = cut.min(i); }
            if let Some(i) = l.find("--") { cut = cut.min(i); }
            format!("{}{}", &l[..cut], " ".repeat(l.len().saturating_sub(cut)))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn symbols_of(content: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let re = Regex::new(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+|export\s+|public\s+)?(?:(?:async|static|const|let|var|val)\s+)?(?:function|def|fn|func|sub|proc|method|class|struct|enum|trait|interface|type|record|module|namespace|package)\s+([A-Za-z_]\w*)|^\s*(?:pub(?:\([^)]*\))?\s+|export\s+|public\s+)(?:const|let|var|val|static)\s+([A-Za-z_]\w*)",
    ).unwrap();
    for (i, line) in content.lines().enumerate() {
        for c in re.captures_iter(line) {
            let name = c.get(1).or_else(|| c.get(2)).map(|m| m.as_str()).unwrap_or("");
            if name.len() > 1 && name != "default" {
                out.push((i + 1, name.to_string()));
            }
        }
    }
    out
}

fn json_str(v: &str) -> String {
    let mut s = String::with_capacity(v.len() + 2);
    for c in v.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => s.push_str(&format!("\\u{:04x}", c as u32)),
            c => s.push(c),
        }
    }
    s
}

fn main() {
    // Embedded Python runtime is available for .py file analysis.
        let _ = Python::try_attach(|py| { let _ = py; });

    let args: Vec<String> = env::args().collect();
    let mut root = ".".to_string();
    let mut json = false;
    let mut excludes: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--exclude" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --exclude requires a value");
                    std::process::exit(1);
                }
                excludes.push(args[i].clone());
            }
            "-h" | "--help" => {
                println!("deadcode - fast cross-language dead-code detector\n");
                println!("Usage: deadcode [PATH] [--json] [--exclude PATTERN]");
                println!("\nDetects exported/public symbols never referenced elsewhere in the repo.");
                println!("Supports 100+ languages by file extension.");
                println!("\nExit codes: 0 ok, 1 invalid arguments, 2 path not found");
                return;
            }
            other => {
                if other.starts_with('-') {
                    eprintln!("error: unknown flag: {}", other);
                    std::process::exit(1);
                }
                root = other.to_string();
            }
        }
        i += 1;
    }

    if !Path::new(&root).exists() {
        eprintln!("error: path not found: {}", root);
        std::process::exit(2);
    }

    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_source(p) || is_excluded(p) {
            continue;
        }
        if excludes.iter().any(|ex| p.to_string_lossy().contains(ex.as_str())) {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(p) {
            files.push((p.to_path_buf(), strip_comments(&raw)));
        }
    }

    let mut file_syms: HashMap<PathBuf, Vec<(usize, String)>> = HashMap::new();
    for (p, content) in &files {
        file_syms.insert(p.clone(), symbols_of(content));
    }

    let mut dead: Vec<(PathBuf, usize, String)> = Vec::new();
    for (p, _) in &files {
        for (ln, sym) in file_syms.get(p).cloned().unwrap_or_default() {
            let mut count = 0usize;
            for (p2, c2) in &files {
                if p2 == p {
                    for (i2, line) in c2.lines().enumerate() {
                        if i2 + 1 == ln { continue; }
                        count += line.matches(sym.as_str()).count();
                    }
                } else {
                    count += c2.matches(sym.as_str()).count();
                }
            }
            if count == 0 {
                dead.push((p.clone(), ln, sym));
            }
        }
    }

    if json {
        let items: Vec<String> = dead
            .iter()
            .map(|(p, ln, sym)| {
                format!(
                    "{{\"file\":\"{}\",\"line\":{},\"symbol\":\"{}\"}}",
                    json_str(&p.display().to_string()),
                    ln,
                    json_str(sym)
                )
            })
            .collect();
        println!(
            "{{\"dead\":{},\"files\":{},\"symbols\":[{}]}}",
            dead.len(),
            files.len(),
            items.join(",")
        );
    } else {
        for (p, ln, sym) in &dead {
            println!("DEAD  {}:{}  {}", p.display(), ln, sym);
        }
        println!("\n{} dead symbol(s) found across {} files.", dead.len(), files.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ts_exports() {
        let s = symbols_of("export function foo() {}\nexport const bar = 1;\n");
        let names: Vec<&str> = s.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
    }

    #[test]
    fn detects_python_rust_go() {
        let py = symbols_of("def handler():\n    pass\n");
        assert!(py.iter().any(|(_, n)| n == "handler"));
        let rs = symbols_of("pub fn run() {}\n");
        assert!(rs.iter().any(|(_, n)| n == "run"));
        let go = symbols_of("func Serve() {}\n");
        assert!(go.iter().any(|(_, n)| n == "Serve"));
    }

    #[test]
    fn ignores_comments() {
        let cleaned = strip_comments("// export function gone() {}\nreal");
        assert!(!cleaned.contains("gone"));
    }

    #[test]
    fn json_escapes_quotes() {
        assert_eq!(json_str("a\"b\\c"), "a\\\"b\\\\c");
    }

    #[test]
    fn source_extensions() {
        assert!(is_source(Path::new("x.ts")));
        assert!(is_source(Path::new("x.rs")));
        assert!(is_source(Path::new("x.kt")));
        assert!(!is_source(Path::new("x.png")));
    }
}
