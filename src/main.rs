pub mod ast;
pub mod cli;
pub mod hooks;
pub mod name_resolution;
pub mod transform;
pub mod type_inferance;
pub mod utils;

use clap::Parser;
use cli::Cli;
use hooks::{
    apply::pre,
    pre::{
        replace_make_with_default, return_slice_to_vec, transform_switches_with_fallthrough_to_if,
        upper_case_string_types,
    },
};
use name_resolution::{Context, Scope};

fn main() {
    let cli = Cli::parse();
    let ast = match gosyn::parse_file(cli.input) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("Failed to parse file, got error: {err}");
            return;
        }
    };
    let mut scope = Scope::new_universe();
    let mut context = Context::new(&mut scope);

    let mut ast = ast.into();
    let hooks = pre::Hooks {
        types: vec![&upper_case_string_types],
        func_types: vec![&return_slice_to_vec],
        statements: vec![&transform_switches_with_fallthrough_to_if],
        expressions: vec![&replace_make_with_default],
    };

    pre::apply(&mut ast, &mut context, &hooks);

    match transform::convert_ast(ast, &mut context) {
        Ok(converted) => {
            if let Some(output) = cli.output {
                if let Err(err) = std::fs::write(output, converted) {
                    eprintln!("Failed to write file: got err {err}");
                }
            } else {
                println!("{converted}");
            }
        }
        Err(err) => {
            eprintln!("Could not convert file, got error: {err}");
        }
    }
}
