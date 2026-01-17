use binaryen_ir::binary_reader::BinaryReader;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wasm-dis")]
#[command(about = "Disassembles WebAssembly binary (.wasm) into text (.wat)", long_about = None)]
struct Args {
    /// Input .wasm file
    input: PathBuf,

    /// Output .wat file
    #[arg(short, long, default_value = "output.wat")]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let allocator = bumpalo::Bump::new();
    let bytes = std::fs::read(&args.input)?;

    let mut reader = BinaryReader::new(&allocator, bytes);
    let module = reader
        .parse_module()
        .map_err(|e| anyhow::anyhow!("Failed to parse binary: {:?}", e))?;

    let wat = module
        .to_wat()
        .map_err(|e| anyhow::anyhow!("Failed to generate WAT: {}", e))?;

    std::fs::write(&args.output, wat)?;
    Ok(())
}
