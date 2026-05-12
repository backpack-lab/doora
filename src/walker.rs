use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

pub fn walk_rust_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|file_type| file_type.is_file()).unwrap_or(false))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .map(|entry| entry.into_path())
}
