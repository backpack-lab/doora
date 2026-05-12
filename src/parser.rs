use std::{cell::RefCell, fs, path::Path};

use tree_sitter::{Parser, Tree};

thread_local! {
    // Each Rayon worker reuses a single Parser instance so we avoid repeated
    // allocation and language setup on every file.
    static RUST_PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::language())
            .expect("failed to configure Rust parser");
        parser
    });
}

pub fn parse_file(path: &Path) -> Option<(Tree, String)> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("warning: failed to read {}: {}", path.display(), error);
            return None;
        }
    };

    let tree = RUST_PARSER.with(|parser_cell| {
        let mut parser = parser_cell.borrow_mut();
        parser.parse(&source, None)
    });

    match tree {
        Some(tree) => Some((tree, source)),
        None => {
            eprintln!("warning: failed to parse {}", path.display());
            None
        }
    }
}
