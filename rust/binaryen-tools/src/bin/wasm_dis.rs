use binaryen_ir::binary_reader::BinaryReader;
use binaryen_tools::{add_feature_flags, apply_feature_flags, read_input, write_output};
use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cmd = Command::new("wasm-dis")
        .about("Disassembles WebAssembly binary (.wasm) into text (.wat)")
        .arg(
            Arg::new("input")
                .help("Input .wasm file")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output .wat file")
                .default_value("output.wat"),
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

    let allocator = bumpalo::Bump::new();
    let bytes = read_input(&input_path)?;

    let mut reader = BinaryReader::new(&allocator, bytes);
    let mut module = reader
        .parse_module()
        .map_err(|e| anyhow::anyhow!("Failed to parse binary: {:?}", e))?;

    apply_feature_flags(&mut module.features, &matches, &feature_flag_ids);

    let wat = module
        .to_wat()
        .map_err(|e| anyhow::anyhow!("Failed to generate WAT: {}", e))?;

    write_output(&output_path, wat.as_bytes())?;
    Ok(())
}
