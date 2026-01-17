use binaryen_ir::binary_writer::BinaryWriter;
use binaryen_ir::module::Module;
use binaryen_ir::validation::Validator;
use binaryen_tools::{add_feature_flags, apply_feature_flags, read_input_string, write_output};
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cmd = Command::new("wasm-as")
        .about("Assembles WebAssembly text (.wat) into binary (.wasm)")
        .arg(
            Arg::new("input")
                .help("Input .wat file")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output .wasm file")
                .default_value("output.wasm"),
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
            Arg::new("all-features")
                .long("all-features")
                .action(ArgAction::SetTrue)
                .help("Enable all features"),
        );

    let (cmd, feature_flag_ids) = add_feature_flags(cmd);
    let matches = cmd.get_matches();

    let input_path: PathBuf = matches.get_one::<String>("input").map(PathBuf::from).unwrap();
    let output_path: PathBuf = matches.get_one::<String>("output").map(PathBuf::from).unwrap();
    let validate = !matches.get_flag("no-validate");

    let allocator = bumpalo::Bump::new();
    let input_text = read_input_string(&input_path)?;

    let mut module = Module::read_wat(&allocator, &input_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse WAT: {}", e))?;

    apply_feature_flags(&mut module.features, &matches, &feature_flag_ids);

    if validate {
        let (valid, errors) = Validator::new(&module).validate();
        if !valid {
            anyhow::bail!("Validation failed:\n{}", errors.join("\n"));
        }
    }

    let mut writer = BinaryWriter::new();
    let bytes = writer
        .write_module(&module)
        .map_err(|e| anyhow::anyhow!("Failed to write binary: {:?}", e))?;

    write_output(&output_path, &bytes)?;
    Ok(())
}
