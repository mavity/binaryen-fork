use anyhow::{anyhow, Context};
use binaryen_ir::binary_reader::BinaryReader;
use binaryen_ir::binary_writer::BinaryWriter;
use binaryen_ir::module::Module;
use binaryen_ir::pass::{OptimizationOptions, PassRunner};
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut cmd = Command::new("wasm-opt")
        .about("Optimizes WebAssembly files")
        .arg(
            Arg::new("input")
                .help("Input file (.wasm or .wat)")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file (.wasm)"),
        )
        .arg(
            Arg::new("debuginfo")
                .short('g')
                .long("debuginfo")
                .action(ArgAction::SetTrue)
                .help("Preserve debug info"),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .action(ArgAction::SetTrue)
                .help("Print debug information"),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .action(ArgAction::SetTrue)
                .help("Validate the module"),
        )
        .arg(
            Arg::new("opt-level")
                .short('O')
                .action(ArgAction::Append)
                .num_args(0..=1)
                .require_equals(true) // Force -O=3 or -O 3? No, we want -O3 too.
                .help("Optimization level (0, 1, 2, 3, 4, s, z)"),
        );

    // Register all passes as long flags
    for name in PassRunner::get_all_pass_names() {
        cmd = cmd.arg(
            Arg::new(name)
                .long(name)
                .action(ArgAction::Append)
                .num_args(0)
                .help(format!("Run the {} pass", name)),
        );
    }

    let matches = cmd.get_matches();

    let input_path: PathBuf = matches
        .get_one::<String>("input")
        .map(PathBuf::from)
        .unwrap();
    let output_path: Option<PathBuf> = matches.get_one::<String>("output").map(PathBuf::from);
    let debug_mode = matches.get_flag("debug");
    let validate = matches.get_flag("validate");

    let mut runner = PassRunner::new();
    runner.set_validate_globally(validate);

    // Collect all pass-related argument indices to preserve order
    let mut actions = Vec::new();

    // Collect Optimization Levels
    if let Some(indices) = matches.indices_of("opt-level") {
        let values: Vec<_> = matches
            .get_many::<String>("opt-level")
            .map(|v| v.collect())
            .unwrap_or_else(Vec::new);

        let mut val_iter = values.into_iter();
        for idx in indices {
            let level = val_iter.next().cloned().unwrap_or_else(|| "2".to_string());
            actions.push((idx, Action::Opt(level)));
        }
    }

    // Collect specific passes
    for name in PassRunner::get_all_pass_names() {
        if let Some(indices) = matches.indices_of(name) {
            for idx in indices {
                actions.push((idx, Action::Pass(name.to_string())));
            }
        }
    }

    // Sort by index to preserve order
    actions.sort_by_key(|(idx, _)| *idx);

    for (_, action) in actions {
        match action {
            Action::Opt(level) => {
                let opt = match level.as_str() {
                    "0" => OptimizationOptions::o0(),
                    "1" => OptimizationOptions::o1(),
                    "2" => OptimizationOptions::o2(),
                    "3" => OptimizationOptions::o3(),
                    "4" => OptimizationOptions::o4(),
                    "s" => OptimizationOptions::os(),
                    "z" => OptimizationOptions::oz(),
                    _ => anyhow::bail!("Unknown optimization level: {}", level),
                };
                runner.add_default_optimization_passes(&opt);
            }
            Action::Pass(name) => {
                if !runner.add_by_name(&name) {
                    anyhow::bail!("Unknown pass: --{}", name);
                }
            }
        }
    }

    let allocator = bumpalo::Bump::new();
    let data = std::fs::read(&input_path)
        .with_context(|| format!("Failed to read input file: {:?}", input_path))?;

    let mut module = if data.starts_with(b"\0asm") {
        let mut reader = BinaryReader::new(&allocator, data);
        reader
            .parse_module()
            .map_err(|e| anyhow!("Binary parse error: {:?}", e))?
    } else {
        let text = std::str::from_utf8(&data).context("WAT input is not valid UTF-8")?;
        Module::read_wat(&allocator, text).map_err(|e| anyhow!("WAT parse error: {}", e))?
    };

    if debug_mode {
        println!("Running passes...");
    }

    runner.run(&mut module);

    if let Some(path) = output_path {
        let mut writer = BinaryWriter::new();
        let bytes = writer
            .write_module(&module)
            .map_err(|e| anyhow!("Write error: {:?}", e))?;
        std::fs::write(path, bytes).context("Failed to write output file")?;
    }

    Ok(())
}

enum Action {
    Opt(String),
    Pass(String),
}
