use binaryen_ir::binary_writer::BinaryWriter;
use binaryen_ir::module::Module;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wasm-as")]
#[command(about = "Assembles WebAssembly text (.wat) into binary (.wasm)", long_about = None)]
struct Args {
    /// Input .wat file
    input: PathBuf,

    /// Output .wasm file
    #[arg(short, long, default_value = "output.wasm")]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let allocator = bumpalo::Bump::new();
    let input_text = std::fs::read_to_string(&args.input)?;

    let module = Module::read_wat(&allocator, &input_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse WAT: {}", e))?;

    let mut writer = BinaryWriter::new();
    let bytes = writer
        .write_module(&module)
        .map_err(|e| anyhow::anyhow!("Failed to write binary: {:?}", e))?;

    std::fs::write(&args.output, bytes)?;
    Ok(())
}
