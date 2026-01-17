use anyhow::{anyhow, Context};
use binaryen_ir::binary_reader::BinaryReader;
use binaryen_ir::binary_writer::BinaryWriter;
use binaryen_ir::module::Module;
use binaryen_ir::pass::{OptimizationOptions, PassRunner, PASS_REGISTRY};
use binaryen_ir::validation::Validator;
use binaryen_ir::wasm_features::FeatureSet;
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
            Arg::new("no-validate")
                .long("no-validate")
                .action(ArgAction::SetTrue)
                .help("Do not validate the module"),
        )
        .arg(
            Arg::new("validate-globally")
                .long("validate-globally")
                .action(ArgAction::SetTrue)
                .help("Validate the module after each pass"),
        )
        .arg(
            Arg::new("opt-level")
                .short('O')
                .action(ArgAction::Append)
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("2")
                .help("Optimization level (0, 1, 2, 3, 4, s, z)"),
        )
        .arg(
            Arg::new("pass-arg")
                .long("pass-arg")
                .action(ArgAction::Append)
                .value_name("KEY=VALUE")
                .help("Pass arguments (e.g. simplify-locals@allow-tee=true)"),
        )
        .arg(
            Arg::new("all-features")
                .long("all-features")
                .action(ArgAction::SetTrue)
                .help("Enable all features"),
        );

    // Register all feature flags
    let mut feature_flag_ids = Vec::new();
    for feature in FeatureSet::iter_all() {
        let name = FeatureSet::to_string(feature);
        let enable_name: &'static str = Box::leak(format!("enable-{}", name).into_boxed_str());
        let disable_name: &'static str = Box::leak(format!("disable-{}", name).into_boxed_str());
        feature_flag_ids.push((feature, enable_name, disable_name));

        cmd = cmd.arg(
            Arg::new(enable_name)
                .long(enable_name)
                .action(ArgAction::SetTrue)
                .help(format!("Enable {} feature", name)),
        );
        cmd = cmd.arg(
            Arg::new(disable_name)
                .long(disable_name)
                .action(ArgAction::SetTrue)
                .help(format!("Disable {} feature", name)),
        );
    }

    // Register all passes as long flags using registry descriptions
    for info in PASS_REGISTRY {
        cmd = cmd.arg(
            Arg::new(info.name)
                .long(info.name)
                .action(ArgAction::Append)
                .num_args(0)
                .help(info.description),
        );
    }

    let matches = cmd.get_matches();

    let input_path: PathBuf = matches
        .get_one::<String>("input")
        .map(PathBuf::from)
        .unwrap();
    let output_path: Option<PathBuf> = matches.get_one::<String>("output").map(PathBuf::from);
    let debug_mode = matches.get_flag("debug");
    let validate = !matches.get_flag("no-validate");
    let validate_globally = matches.get_flag("validate-globally");

    let mut runner = PassRunner::new();
    runner.set_validate_globally(validate_globally);

    // Collect pass arguments
    if let Some(args) = matches.get_many::<String>("pass-arg") {
        for arg in args {
            if let Some((key, val)) = arg.split_once('=') {
                runner.pass_args.insert(key.to_string(), val.to_string());
            }
        }
    }

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

    // Apply features from flags
    if matches.get_flag("all-features") {
        module.features = FeatureSet::ALL;
    }

    for (feature, enable_id, disable_id) in feature_flag_ids {
        if matches.get_flag(enable_id) {
            module.features.enable(feature);
        }
        if matches.get_flag(disable_id) {
            module.features.disable(feature);
        }
    }

    if debug_mode {
        println!("Running passes...");
    }

    runner.run(&mut module);

    if validate {
        let (valid, errors) = Validator::new(&module).validate();
        if !valid {
            anyhow::bail!("Validation failed:\n{}", errors.join("\n"));
        }
    }

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
