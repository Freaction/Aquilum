use std::collections::HashMap;

use crate::functions::FunctionSpec;

pub fn register(_map: &mut HashMap<&'static str, FunctionSpec>) {
    // \verb is handled directly in Parser::parse_symbol_inner
    // because it has special lexing requirements.
}
