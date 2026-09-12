const LINE_OVERHEAD_TOKENS: f64 = 1.03;
const TOKENS_PER_VISIBLE_CHAR: f64 = 0.357;

pub fn line_tokens(visible_chars: usize) -> f64 {
    LINE_OVERHEAD_TOKENS + TOKENS_PER_VISIBLE_CHAR * visible_chars as f64
}
