#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

//! Language-specific symbol extraction using Tree-sitter queries.
//!
//! `SymbolExtractor` runs a small set of language queries to discover
//! top-level symbols (functions, types, imports) and converts them into
//! `NewSymbolRow` structures suitable for insertion into the memory DB.

use crate::memory::{NewSymbolRow, SymbolKind};
use crate::parser::FileSource;
use crate::types::Language;
use std::fmt;
use tree_sitter::{Language as TsLanguage, Node, Query, QueryCursor, Tree};

#[derive(Debug, Clone)]
pub struct SymbolExtractor {
    /// The language to extract symbols for.
    pub language: Language,
}

impl SymbolExtractor {
    /// Extract symbols from `tree` and `source` and associate them with `file_id`.
    ///
    /// Returns a vector of `NewSymbolRow` describing the discovered symbols.
    pub fn extract(&self, tree: &Tree, source: &FileSource, file_id: i64) -> Vec<NewSymbolRow> {
        let queries = queries_for_language(&self.language);
        let ts_language = ts_language_for(&self.language);
        let mut symbols = Vec::new();

        if let Some(query) = queries.functions {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Function,
            ));
        }

        if let Some(query) = queries.structs {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Struct,
            ));
        }

        if let Some(query) = queries.classes {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Class,
            ));
        }

        if let Some(query) = queries.interfaces {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Interface,
            ));
        }

        if let Some(query) = queries.type_aliases {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::TypeAlias,
            ));
        }

        if let Some(query) = queries.imports {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Import,
            ));
        }

        if let Some(query) = queries.traits {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Trait,
            ));
        }

        if let Some(query) = queries.enums {
            symbols.extend(extract_with_query(
                tree,
                source,
                file_id,
                &ts_language,
                query,
                SymbolKind::Enum,
            ));
        }

        symbols
    }
}

struct LangQueries {
    functions: Option<&'static str>,
    structs: Option<&'static str>,
    classes: Option<&'static str>,
    interfaces: Option<&'static str>,
    type_aliases: Option<&'static str>,
    imports: Option<&'static str>,
    traits: Option<&'static str>,
    enums: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct CaptureData {
    text: String,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

impl fmt::Display for CaptureData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

fn extract_with_query(
    tree: &Tree,
    source: &FileSource,
    file_id: i64,
    ts_lang: &TsLanguage,
    query_str: &str,
    #[allow(clippy::needless_pass_by_value)] kind: SymbolKind,
) -> Vec<NewSymbolRow> {
    let query = Query::new(ts_lang, query_str).expect("invalid extractor query");
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut symbols = Vec::new();

    for query_match in cursor.matches(&query, tree.root_node(), source.as_bytes()) {
        let mut name_capture: Option<CaptureData> = None;
        let mut trait_name_capture: Option<CaptureData> = None;
        let mut type_name_capture: Option<CaptureData> = None;
        let mut signature: Option<String> = None;

        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize].to_string();
            match capture_name.as_str() {
                "name" => name_capture = capture_data(source, capture.node),
                "trait_name" => trait_name_capture = capture_data(source, capture.node),
                "type_name" => type_name_capture = capture_data(source, capture.node),
                "sig" => signature = capture_signature(source, capture.node),
                _ => {}
            }
        }

        let (name, start_line, start_col, end_line, end_col) = match kind {
            SymbolKind::Trait if name_capture.is_none() => {
                let trait_name = match trait_name_capture.clone() {
                    Some(data) => data,
                    None => continue,
                };
                let type_name = match type_name_capture.clone() {
                    Some(data) => data,
                    None => continue,
                };
                (
                    format!("{} for {}", trait_name.text, type_name.text),
                    trait_name.start_line,
                    trait_name.start_col,
                    type_name.end_line,
                    type_name.end_col,
                )
            }
            _ => {
                let capture = match name_capture.clone().or(trait_name_capture.clone()) {
                    Some(data) => data,
                    None => continue,
                };
                (
                    capture.text,
                    capture.start_line,
                    capture.start_col,
                    capture.end_line,
                    capture.end_col,
                )
            }
        };

        symbols.push(NewSymbolRow {
            file_id,
            kind: kind.clone(),
            name,
            start_line,
            start_col,
            end_line,
            end_col,
            signature,
        });
    }

    symbols
}

fn capture_data(source: &FileSource, node: Node<'_>) -> Option<CaptureData> {
    let bytes = source.as_bytes().get(node.byte_range())?;
    let text = std::str::from_utf8(bytes).ok()?.to_string();
    let start = node.start_position();
    let end = node.end_position();
    Some(CaptureData {
        text,
        start_line: start.row + 1,
        start_col: start.column,
        end_line: end.row + 1,
        end_col: end.column,
    })
}

fn capture_signature(source: &FileSource, node: Node<'_>) -> Option<String> {
    let text = capture_data(source, node)?.text;
    Some(truncate_signature(&text))
}

fn truncate_signature(text: &str) -> String {
    let count = text.chars().count();
    if count <= 500 {
        text.to_string()
    } else {
        let mut truncated = text.chars().take(500).collect::<String>();
        truncated.push('…');
        truncated
    }
}

fn ts_language_for(language: &Language) -> TsLanguage {
    match language {
        Language::Rust => crate::parser::get_language("rust").expect("missing rust language"),
        Language::Python => crate::parser::get_language("python").expect("missing python language"),
        Language::JavaScript => {
            crate::parser::get_language("js").expect("missing javascript language")
        }
        Language::TypeScript => {
            crate::parser::get_language("ts").expect("missing typescript language")
        }
        Language::Go => crate::parser::get_language("go").expect("missing go language"),
        Language::C => crate::parser::get_language("c").expect("missing c language"),
        Language::Cpp => crate::parser::get_language("cpp").expect("missing cpp language"),
    }
}

fn queries_for_language(lang: &Language) -> LangQueries {
    match lang {
        Language::Rust => LangQueries {
            functions: Some("(source_file (function_item name: (identifier) @name) @sig)"),
            structs: Some("(struct_item name: (type_identifier) @name) @sig"),
            classes: None,
            interfaces: None,
            type_aliases: Some("(type_item name: (type_identifier) @name) @sig"),
            imports: Some("(use_declaration (_) @name) @sig"),
            traits: Some(
                "(trait_item name: (type_identifier) @name) @sig\n(impl_item trait: (_) @trait_name type: (_) @type_name) @sig",
            ),
            enums: Some("(enum_item name: (type_identifier) @name) @sig"),
        },
        Language::Python => LangQueries {
            functions: Some("(function_definition name: (identifier) @name) @sig"),
            structs: None,
            classes: Some("(class_definition name: (identifier) @name) @sig"),
            interfaces: None,
            type_aliases: None,
            imports: Some(
                "(import_statement (dotted_name) @name) @sig\n(import_from_statement module_name: (dotted_name) @name) @sig",
            ),
            traits: None,
            enums: None,
        },
        Language::JavaScript => LangQueries {
            functions: Some("(function_declaration name: (identifier) @name) @sig"),
            structs: None,
            classes: Some("(class_declaration name: (identifier) @name) @sig"),
            interfaces: None,
            type_aliases: None,
            imports: Some("(import_statement source: (string) @name) @sig"),
            traits: None,
            enums: None,
        },
        Language::TypeScript => LangQueries {
            functions: Some("(function_declaration name: (identifier) @name) @sig"),
            structs: None,
            classes: Some("(class_declaration name: (type_identifier) @name) @sig"),
            interfaces: Some("(interface_declaration name: (type_identifier) @name) @sig"),
            type_aliases: Some("(type_alias_declaration name: (type_identifier) @name) @sig"),
            imports: Some("(import_statement source: (string) @name) @sig"),
            traits: None,
            enums: None,
        },
        Language::Go => LangQueries {
            functions: Some("(function_declaration name: (identifier) @name) @sig"),
            structs: Some("(type_declaration (type_spec name: (type_identifier) @name) @sig)"),
            classes: None,
            interfaces: None,
            type_aliases: None,
            imports: Some("(import_declaration (import_spec path: (interpreted_string_literal) @name) @sig)"),
            traits: None,
            enums: None,
        },
        Language::C => LangQueries {
            functions: Some(
                "(function_definition declarator: (function_declarator declarator: (identifier) @name) @sig)",
            ),
            structs: Some(
                "(type_definition declarator: (type_identifier) @name) @sig\n(struct_specifier name: (type_identifier) @name) @sig",
            ),
            classes: None,
            interfaces: None,
            type_aliases: None,
            imports: Some("(preproc_include path: (_) @name) @sig"),
            traits: None,
            enums: Some("(enum_specifier name: (type_identifier) @name) @sig"),
        },
        Language::Cpp => LangQueries {
            functions: Some(
                "(function_definition declarator: (function_declarator declarator: (identifier) @name) @sig)",
            ),
            structs: Some("(struct_specifier name: (type_identifier) @name) @sig"),
            classes: Some("(class_specifier name: (type_identifier) @name) @sig"),
            interfaces: None,
            type_aliases: None,
            imports: Some("(preproc_include path: (_) @name) @sig"),
            traits: None,
            enums: Some("(enum_specifier name: (type_identifier) @name) @sig"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::get_language;
    use tree_sitter::Parser;

    fn parse_source(language: &str, source: &str) -> (Tree, FileSource) {
        let ts_language = get_language(language).unwrap();
        let mut parser = Parser::new();
        parser.set_language(&ts_language).unwrap();
        let tree = parser.parse(source.as_bytes(), None).unwrap();
        (tree, FileSource::Heap(source.to_string()))
    }

    fn extract(language: Language, source: &str) -> Vec<NewSymbolRow> {
        let (tree, file_source) = parse_source(
            match language {
                Language::Rust => "rust",
                Language::Python => "python",
                Language::JavaScript => "js",
                Language::TypeScript => "ts",
                Language::Go => "go",
                Language::C => "c",
                Language::Cpp => "cpp",
            },
            source,
        );
        let extractor = SymbolExtractor { language };
        extractor.extract(&tree, &file_source, 7)
    }

    #[test]
    fn test_extract_rust_functions() {
        let rows = extract(Language::Rust, "fn alpha() {}\nfn beta() {}");
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.kind == SymbolKind::Function));
        let names: Vec<_> = rows.iter().map(|row| row.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_extract_rust_struct() {
        let rows = extract(Language::Rust, "struct Config { timeout: u64 }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Struct);
        assert_eq!(rows[0].name, "Config");
    }

    #[test]
    fn test_extract_rust_enum() {
        let rows = extract(Language::Rust, "enum Status { Active, Inactive }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Enum);
        assert_eq!(rows[0].name, "Status");
    }

    #[test]
    fn test_extract_rust_trait_impl() {
        let rows = extract(Language::Rust, "impl core::fmt::Display for Config { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { Ok(()) } }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Trait);
        assert!(rows[0].name.contains("Display"));
        assert!(rows[0].name.contains("Config"));
    }

    #[test]
    fn test_extract_python_functions() {
        let rows =
            extract(Language::Python, "def greet(name):\n    pass\ndef farewell():\n    pass");
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.kind == SymbolKind::Function));
    }

    #[test]
    fn test_extract_python_class() {
        let rows = extract(Language::Python, "class MyClass:\n    pass");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Class);
        assert_eq!(rows[0].name, "MyClass");
    }

    #[test]
    fn test_extract_javascript_function() {
        let rows = extract(Language::JavaScript, "function authenticate(user) { return true; }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_typescript_interface() {
        let rows = extract(Language::TypeScript, "interface Shape { area(): number; }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Interface);
        assert_eq!(rows[0].name, "Shape");
    }

    #[test]
    fn test_extract_typescript_type_alias() {
        let rows = extract(Language::TypeScript, "type Point = { x: number; y: number; };\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::TypeAlias);
        assert_eq!(rows[0].name, "Point");
    }

    #[test]
    fn test_extract_go_function() {
        let rows =
            extract(Language::Go, "package main\nfunc greet(name string) string { return name }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_c_function() {
        let rows = extract(Language::C, "int add(int a, int b) { return a + b; }");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Function);
        assert_eq!(rows[0].name, "add");
    }

    #[test]
    fn test_extract_cpp_class() {
        let rows = extract(Language::Cpp, "class Calculator { public: int add(int a, int b); };\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SymbolKind::Class);
        assert_eq!(rows[0].name, "Calculator");
    }

    #[test]
    fn test_signature_truncated_at_500_chars() {
        let long_params = (0..120).map(|i| format!("value{i}: i32")).collect::<Vec<_>>().join(", ");
        let source = format!("fn huge({long_params}) {{}}\n");
        let rows = extract(Language::Rust, &source);
        assert_eq!(rows.len(), 1);
        let signature = rows[0].signature.as_ref().unwrap();
        assert!(signature.chars().count() <= 501);
    }

    #[test]
    fn test_extract_empty_source_returns_empty() {
        let rows = extract(Language::Rust, "let x = 1;");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_extract_returns_correct_positions() {
        let rows = extract(Language::Rust, "fn foo() {}");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].start_line, 1);
        assert_eq!(rows[0].start_col, 3);
    }
}
