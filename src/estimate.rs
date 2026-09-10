const LINE_OVERHEAD_TOKENS: f64 = 2.82;
const TOKENS_PER_CHAR: f64 = 0.2741;

pub fn line_tokens(chars: usize) -> f64 {
    LINE_OVERHEAD_TOKENS + TOKENS_PER_CHAR * chars as f64
}
