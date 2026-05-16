use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// See docs/wiki/systems/문서-규칙-테스트.md for why document tests must link back to wiki rules.
fn main() {
    let repo_root = find_repo_root().unwrap_or_else(|| {
        eprintln!("failed to find repository root containing docs/raw and docs/wiki");
        process::exit(2);
    });

    let docs = collect_markdown_docs(&repo_root);
    let mut inbound: BTreeMap<PathBuf, usize> = docs.iter().map(|path| (path.clone(), 0)).collect();
    let by_stem = index_by_stem(&docs);

    for source in &docs {
        let text = fs::read_to_string(source).unwrap_or_else(|err| {
            eprintln!("failed to read {}: {err}", display_path(&repo_root, source));
            process::exit(2);
        });

        for target in markdown_link_targets(source, &text) {
            if docs.contains(&target) && target != *source {
                *inbound.entry(target).or_insert(0) += 1;
            }
        }

        for stem in wiki_link_targets(&text) {
            if let Some(targets) = by_stem.get(&stem) {
                for target in targets {
                    if target != source {
                        *inbound.entry(target.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let orphans: Vec<_> = inbound
        .iter()
        .filter_map(|(path, count)| (*count == 0).then_some(path))
        .collect();

    if !orphans.is_empty() {
        eprintln!("orphan documents found:");
        for path in orphans {
            eprintln!("- {}", display_path(&repo_root, path));
        }
        process::exit(1);
    }

    println!("ok: no orphan documents found");
}

fn find_repo_root() -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    for dir in current.ancestors() {
        if dir.join("docs/raw").is_dir() && dir.join("docs/wiki").is_dir() {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn collect_markdown_docs(repo_root: &Path) -> BTreeSet<PathBuf> {
    let mut docs = BTreeSet::new();
    collect_markdown_docs_in(&repo_root.join("docs/raw"), &mut docs);
    collect_markdown_docs_in(&repo_root.join("docs/wiki"), &mut docs);
    docs
}

fn collect_markdown_docs_in(dir: &Path, docs: &mut BTreeSet<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| {
        eprintln!("failed to read directory {}: {err}", dir.display());
        process::exit(2);
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| {
                eprintln!("failed to read directory entry in {}: {err}", dir.display());
                process::exit(2);
            })
            .path();

        if path.is_dir() {
            collect_markdown_docs_in(&path, docs);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            docs.insert(path);
        }
    }
}

fn index_by_stem(docs: &BTreeSet<PathBuf>) -> BTreeMap<String, Vec<PathBuf>> {
    let mut by_stem: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for doc in docs {
        if let Some(stem) = doc.file_stem().and_then(|stem| stem.to_str()) {
            by_stem
                .entry(stem.to_string())
                .or_default()
                .push(doc.clone());
        }
    }
    by_stem
}

fn markdown_link_targets(source: &Path, text: &str) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while let Some(open) = find_bytes(bytes, b"](", index) {
        let target_start = open + 2;
        if let Some(close_offset) = text[target_start..].find(')') {
            let raw_target = &text[target_start..target_start + close_offset];
            if let Some(target) = normalize_markdown_target(source, raw_target) {
                targets.push(target);
            }
            index = target_start + close_offset + 1;
        } else {
            break;
        }
    }

    targets
}

fn normalize_markdown_target(source: &Path, raw_target: &str) -> Option<PathBuf> {
    let without_anchor = raw_target.split('#').next().unwrap_or(raw_target).trim();
    if !without_anchor.ends_with(".md") {
        return None;
    }

    let trimmed = without_anchor.trim_matches(['<', '>']);
    let parent = source.parent()?;
    Some(normalize_path(parent.join(trimmed)))
}

fn wiki_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };

        let raw_target = &after_start[..end];
        let target = raw_target
            .split('|')
            .next()
            .unwrap_or(raw_target)
            .split('#')
            .next()
            .unwrap_or(raw_target)
            .trim();

        if !target.is_empty() {
            targets.push(target.to_string());
        }

        rest = &after_start[end + 2..];
    }

    targets
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
