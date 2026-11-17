use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn is_source(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()), Some("js" | "ts" | "tsx" | "jsx" | "py" | "rs" | "go"))
}

fn exports_of(content: &str, ext: &str) -> Vec<String> {
    let mut out = Vec::new();
    match ext {
        "js" | "ts" | "tsx" | "jsx" => {
            let re = Regex::new(r"(?m)export\s+(?:default\s+)?(?:function|const|let|var|class|interface|type|enum)\s+([A-Za-z_]\w*)").unwrap();
            for c in re.captures_iter(content) { out.push(c[1].to_string()); }
            let re2 = Regex::new(r"(?m)export\s*\{\s*([^}]+)\s*\}").unwrap();
            for c in re2.captures_iter(content) {
                for part in c[1].split(',') {
                    let name = part.trim().split_whitespace().next().unwrap_or("").trim_matches('"');
                    if !name.is_empty() && name != "default" { out.push(name.to_string()); }
                }
            }
        }
        "py" => {
            let re = Regex::new(r"(?m)^def\s+([A-Za-z_]\w*)\s*\(|^class\s+([A-Za-z_]\w*)").unwrap();
            for c in re.captures_iter(content) {
                if let Some(m) = c.get(1).or(c.get(2)) { out.push(m.as_str().to_string()); }
            }
        }
        "rs" => {
            let re = Regex::new(r"(?m)pub\s+(?:fn|struct|enum|trait|const|static|type|mod)\s+([A-Za-z_]\w*)").unwrap();
            for c in re.captures_iter(content) { out.push(c[1].to_string()); }
        }
        "go" => {
            let re = Regex::new(r"(?m)^func\s+(?:\([^)]*\)\s+)?([A-Z]\w*)").unwrap();
            for c in re.captures_iter(content) { out.push(c[1].to_string()); }
        }
        _ => {}
    }
    out
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let mut files: Vec<(String, String, String)> = Vec::new(); // path, ext, content
    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_source(p) { continue; }
        // skip node_modules, target, .git
        if p.components().any(|c| c.as_os_str() == "node_modules" || c.as_os_str() == "target" || c.as_os_str() == ".git") { continue; }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        if let Ok(content) = fs::read_to_string(p) {
            files.push((p.display().to_string(), ext, content));
        }
    }

    // collect all exported symbols per file
    let mut exports: HashMap<String, Vec<String>> = HashMap::new(); // file -> symbols
    let mut all_content = String::new();
    for (path, ext, content) in &files {
        exports.insert(path.clone(), exports_of(content, ext));
        all_content.push_str(content);
        all_content.push('\n');
    }

    // a symbol is "used" if it appears anywhere other than its own file's export line
    let mut used: HashSet<String> = HashSet::new();
    for (_, _, content) in &files {
        // count occurrences of each candidate in this file's content
        for sym in exports.values().flatten() {
            // crude: if symbol appears more than once globally, it's used
            let count = all_content.matches(sym.as_str()).count();
            if count > 1 { used.insert(sym.clone()); }
        }
    }

    let mut dead_count = 0;
    for (path, _, _) in &files {
        for sym in exports.get(path).cloned().unwrap_or_default() {
            if !used.contains(&sym) {
                println!("DEAD: {} :: {}", path, sym);
                dead_count += 1;
            }
        }
    }
    println!("\n{} dead symbol(s) found across {} files.", dead_count, files.len());
}
