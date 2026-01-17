use anyhow::{Context, Result};
use binaryen_ir::module::Module;
use binaryen_ir::pass::{OptimizationOptions, PassRunner};
use binaryen_tools::{add_feature_flags, apply_feature_flags, read_input_string};
use clap::{Arg, Command};
use std::path::PathBuf;
use wast::core::ModuleKind;
use wast::WastDirective;

fn main() -> Result<()> {
    let cmd = Command::new("rust-lit-adapter")
        .about("Lit adapter for Binaryen Rust port")
        .arg(Arg::new("input").help("Input .wast file").required(true))
        .arg(
            Arg::new("passes")
                .short('p')
                .long("passes")
                .help("Comma-separated list of passes to run"),
        );

    let (cmd, feature_flag_ids) = add_feature_flags(cmd);
    let matches = cmd.get_matches();

    let input_path: PathBuf = matches
        .get_one::<String>("input")
        .map(PathBuf::from)
        .unwrap();
    let passes_str = matches.get_one::<String>("passes");

    let input_text = read_input_string(&input_path)?;
    let allocator = bumpalo::Bump::new();

    // Parse WAST
    let buf = wast::parser::ParseBuffer::new(&input_text)
        .map_err(|e| anyhow::anyhow!("Failed to create parse buffer: {}", e))?;
    let wast = wast::parser::parse::<wast::Wast>(&buf)
        .map_err(|e| anyhow::anyhow!("Failed to parse WAST: {}", e))?;

    for directive in wast.directives {
        match directive {
            WastDirective::Module(mut module) => {
                let wasm_binary = module
                    .encode()
                    .map_err(|e| anyhow::anyhow!("Failed to encode module: {}", e))?;

                let mut rust_module = Module::read_binary(&allocator, &wasm_binary)
                    .map_err(|e| anyhow::anyhow!("Failed to read binary into IR: {:?}", e))?;

                apply_feature_flags(&mut rust_module.features, &matches, &feature_flag_ids);

                if let Some(p_str) = passes_str {
                    let mut runner = PassRunner::new();
                    for name in p_str.split(',') {
                        if !runner.add_by_name(name.trim()) {
                            anyhow::bail!("Unknown pass: {}", name);
                        }
                    }
                    runner.run(&mut rust_module);
                }

                // Output the result in text format (standard for lit tests)
                println!(
                    "{}",
                    rust_module
                        .to_wat()
                        .map_err(|e| anyhow::anyhow!("Failed to convert to WAT: {}", e))?
                );
            }
            _ => {
                // Ignore other directives (assertions) for now as we are focusing on pass parity
            }
        }
    }

    Ok(())
}
