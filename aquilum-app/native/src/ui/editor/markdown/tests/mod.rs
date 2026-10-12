mod block;
mod inline;
mod list;
mod table;

use super::blocks::{Block, classify};

fn blocks(text: &str) -> Vec<Block> {
    classify(text.split('\n'))
}
