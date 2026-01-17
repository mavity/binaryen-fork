use binaryen_ir::binary_reader::BinaryReader;
use binaryen_ir::binary_writer::BinaryWriter;
use binaryen_ir::module::Module;
use binaryen_ir::pass::{OptimizationOptions, PassRunner};
use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "wasm-opt")]
#[command(about = "Optimizes WebAssembly binary/text files", long_about = None)]
struct Args {
    /// Input file (.wasm or .wat)
    input: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optimization level (0, 1, 2, 3, 4)
    #[arg(short = 'O', default_value = "0")]
    optimize_level: u32,

    /// Shrink level (0, 1, 2)
    #[arg(short = 's', action = ArgAction::Count)]
    shrink_level: u8,

    /// Optimize for size (-Os)
    #[arg(long)]
    os: bool,

    /// Optimize aggressively for size (-Oz)
    #[arg(long)]
    oz: bool,

    /// Run passes in debug mode
    #[arg(short, long)]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let allocator = bumpalo::Bump::new();
    let data = std::fs::read(&args.input)?;

    // Try reading as binary first, then as WAT
    let mut module = if data.starts_with(b"\0asm") {
        let mut reader = BinaryReader::new(&allocator, data);
        reader
            .parse_module()
            .map_err(|e| anyhow::anyhow!("Binary parse error: {:?}", e))?
    } else {
        let text = std::str::from_utf8(&data)?;
        Module::read_wat(&allocator, text).map_err(|e| anyhow::anyhow!("WAT parse error: {}", e))?
    };

    let mut options = if args.oz {
        OptimizationOptions::oz()
    } else if args.os {
        OptimizationOptions::os()
    } else {
        match args.optimize_level {
            0 => OptimizationOptions::o0(),
            1 => OptimizationOptions::o1(),
            2 => OptimizationOptions::o2(),
            3 => OptimizationOptions::o3(),
            4 => OptimizationOptions::o4(),
            _ => OptimizationOptions::o0(),
        }
    };

    // Manual overrides from numeric flags
    if args.optimize_level > 0 {
        options.optimize_level = args.optimize_level;
    }
    if args.shrink_level > 0 {
        options.shrink_level = args.shrink_level as u32;
    }
    options.debug = args.debug;

    let mut runner = PassRunner::new();
    runner.add_default_optimization_passes(&options);
    runner.run(&mut module);

    if let Some(output_path) = args.output {
        let mut writer = BinaryWriter::new();
        let bytes = writer
            .write_module(&module)
            .map_err(|e| anyhow::anyhow!("Write error: {:?}", e))?;
        std::fs::write(output_path, bytes)?;
    }

    Ok(())
}
