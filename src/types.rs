#[derive(Debug, Clone)]
pub struct MatchResult {
    pub file_path: String,
    pub capture_name: String,
    pub matched_text: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}
