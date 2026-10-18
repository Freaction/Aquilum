mod block;
mod inline;
mod list;
mod table;

use super::blocks::{Block, Frame, classify};

fn blocks(text: &str) -> Vec<Block> {
    classify(text.split('\n')).into_iter().map(|(_, block)| block).collect()
}

fn frames(text: &str) -> Vec<Frame> {
    classify(text.split('\n')).into_iter().map(|(frame, _)| frame).collect()
}
