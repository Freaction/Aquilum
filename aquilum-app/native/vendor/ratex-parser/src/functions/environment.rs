use std::collections::HashMap;

use crate::environments::{EnvContext, ENVIRONMENTS};
use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\begin", "\\end"],
        "environment",
        1,
        0,
        Some(vec![ArgType::Text]),
        true,
        true,
        true,
        false,
        false,
        handle_begin_end,
    );

    define_function_full(
        map,
        &["\\hline", "\\hdashline"],
        "text",
        0,
        0,
        None,
        false,
        true,
        true,
        false,
        false,
        handle_hline,
    );
}

fn extract_env_name(name_group: &ParseNode) -> ParseResult<String> {
    match name_group {
        ParseNode::OrdGroup { body, .. } => {
            let mut env_name = String::new();
            for node in body {
                match node {
                    ParseNode::TextOrd { text, .. } | ParseNode::MathOrd { text, .. } => {
                        env_name.push_str(text);
                    }
                    ParseNode::Atom { text, .. } => {
                        env_name.push_str(text);
                    }
                    _ => {
                        return Err(ParseError::msg(format!(
                            "Invalid environment name character: {:?}",
                            node.type_name()
                        )));
                    }
                }
            }
            Ok(env_name)
        }
        _ => Err(ParseError::msg("Invalid environment name")),
    }
}

fn handle_begin_end(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let name_group = &args[0];
    let env_name = extract_env_name(name_group)?;

    if ctx.func_name == "\\begin" {
        // Allow all registered environments to parse; display vs inline is a layout concern.
        // (KaTeX rejects align/equation in inline $...$; we parse them and let layout decide.)
        let env = ENVIRONMENTS
            .get(env_name.as_str())
            .ok_or_else(|| ParseError::msg(format!("No such environment: {}", env_name)))?;

        // Parse environment arguments if any
        let mut env_args = Vec::new();
        for _ in 0..env.num_args {
            let arg = ctx.parser.parse_argument_group(false, None)?;
            match arg {
                Some(a) => env_args.push(a),
                None => {
                    return Err(ParseError::msg(format!(
                        "Expected argument to \\begin{{{}}}",
                        env_name
                    )));
                }
            }
        }

        let mut env_ctx = EnvContext {
            mode: ctx.parser.mode,
            env_name: env_name.clone(),
            parser: ctx.parser,
        };

        let result = (env.handler)(&mut env_ctx, env_args, Vec::new())?;

        // Expect \end
        ctx.parser.expect("\\end", false)?;

        // Parse \end{name} as a function call
        let end_node = ctx.parser.parse_function(None, None)?;
        let end_name = match &end_node {
            Some(ParseNode::Environment { name, .. }) => name.clone(),
            _ => {
                return Err(ParseError::msg("Expected \\end after environment body"));
            }
        };

        if end_name != env_name {
            return Err(ParseError::msg(format!(
                "Mismatch: \\begin{{{}}} matched by \\end{{{}}}",
                env_name, end_name
            )));
        }

        Ok(result)
    } else {
        // \end handler
        Ok(ParseNode::Environment {
            mode: ctx.parser.mode,
            name: env_name,
            name_group: Box::new(name_group.clone()),
            loc: None,
        })
    }
}

fn handle_hline(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Err(ParseError::msg(format!(
        "{} valid only within array environment",
        ctx.func_name
    )))
}
