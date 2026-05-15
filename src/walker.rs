use std::path::Path;

use ignore::{DirEntry, WalkBuilder};

use crate::types::{AppError, Language, Result};

const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    ".tox",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "build",
    ".idea",
    ".vscode",
];

fn is_excluded_dir(entry: &DirEntry) -> bool {
    entry
        .file_type()
        .map_or(false, |ft| ft.is_dir())
        && entry
            .file_name()
            .to_str()
            .map_or(false, |name| EXCLUDED_DIRS.contains(&name))
}

fn is_binary(path: &Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return true;
    };

    let mut buffer = [0u8; 8192];
    let Ok(n) = file.read(&mut buffer) else {
        return true;
    };

    let sample = &buffer[..n];

    if sample.contains(&0u8) {
        return true;
    }

    std::str::from_utf8(sample).is_err()
}

pub fn extensions_for_language(lang: &Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["rs"],
        Language::Python => &["py", "pyi"],
        Language::JavaScript => &["js", "mjs", "cjs"],
        Language::TypeScript => &["ts", "mts", "cts", "tsx"],
        Language::Go => &["go"],
    }
}

pub fn build_walker(root: &Path, lang: &Language) -> impl Iterator<Item = Result<DirEntry>> {
    let extensions = extensions_for_language(lang);

    WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .follow_links(false)
        .filter_entry(|entry| !is_excluded_dir(entry))
        .build()
        .filter_map(move |result| match result {
            Err(error) => Some(Err(AppError::WalkError(error))),
            Ok(entry) => {
                let is_file = entry.file_type().map_or(false, |ft| ft.is_file());
                let ext_matches = entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extensions.contains(&extension.to_lowercase().as_str()))
                    .unwrap_or(false);

                if is_file && ext_matches && !is_binary(entry.path()) {
                    Some(Ok(entry))
                } else {
                    None
                }
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- is_excluded_dir tests ---

    #[test]
    fn excluded_dirs_are_not_walked() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // create excluded dirs with files inside
        for excluded in &["target", "node_modules", ".git", "__pycache__"] {
            let d = root.join(excluded);
            fs::create_dir(&d).unwrap();
            fs::write(d.join("file.rs"), "fn foo() {}").unwrap();
        }

        // create a normal file at root
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();

        let names: Vec<String> = build_walker(root, &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(names.contains(&"main.rs".to_string()),
            "main.rs should be found");
        assert!(!names.contains(&"file.rs".to_string()),
            "files inside excluded dirs should not appear");
    }

    #[test]
    fn non_excluded_dirs_are_walked() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("lib.rs"), "").unwrap();

        let names: Vec<String> = build_walker(root, &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(names.contains(&"lib.rs".to_string()));
    }

    // --- is_binary tests ---

    #[test]
    fn binary_files_are_excluded() {
        let dir = TempDir::new().unwrap();
        // file with null byte — binary signal
        let binary_content = b"fn main() \x00 {}";
        fs::write(dir.path().join("sneaky.rs"), binary_content).unwrap();

        let names: Vec<String> = build_walker(dir.path(), &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(!names.contains(&"sneaky.rs".to_string()),
            "File with null byte should be treated as binary and excluded");
    }

    #[test]
    fn valid_utf8_files_are_included() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("valid.rs"), "fn main() { println!(\"héllo\"); }").unwrap();

        let names: Vec<String> = build_walker(dir.path(), &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(names.contains(&"valid.rs".to_string()),
            "Valid UTF-8 file with non-ASCII chars should be included");
    }

    #[test]
    fn empty_file_is_not_binary() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("empty.rs"), b"").unwrap();

        let names: Vec<String> = build_walker(dir.path(), &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(names.contains(&"empty.rs".to_string()),
            "Empty file should not be treated as binary");
    }

    #[test]
    fn invalid_utf8_file_is_excluded() {
        let dir = TempDir::new().unwrap();
        // invalid UTF-8 sequence
        let bad_bytes: &[u8] = &[0x66, 0x6E, 0x20, 0x80, 0x81, 0x82];
        fs::write(dir.path().join("bad.rs"), bad_bytes).unwrap();

        let names: Vec<String> = build_walker(dir.path(), &Language::Rust)
            .collect::<Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.path().file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        assert!(!names.contains(&"bad.rs".to_string()),
            "Invalid UTF-8 file should be excluded");
    }
}
