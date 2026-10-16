use tantivy::tokenizer::{AsciiFoldingFilter, LowerCaser, RemoveLongFilter, TextAnalyzer};

use super::filters::{ExclusionFilter, SanitizingTokenizer};
use super::morphology::StemFilter;

pub const TOKENIZER_NAME: &str = "aquilum_text";
pub const ANALYZER_VERSION: u32 = 11;

pub fn indexing_tokenizer() -> TextAnalyzer {
    TextAnalyzer::builder(SanitizingTokenizer::default())
        .filter(RemoveLongFilter::limit(120))
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .filter(StemFilter)
        .filter(ExclusionFilter)
        .build()
}
