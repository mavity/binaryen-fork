use binaryen_decompile::{CPrinter, Lifter, RustPrinter};
use binaryen_ir::BinaryReader;
use bumpalo::Bump;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_triad_test(source_path_str: &str) {
    let source_path = hex_to_path(source_path_str);
    let stem = source_path
        .file_stem()
        .expect("file stem")
        .to_str()
        .expect("utf8 stem");
    let dir = source_path.parent().expect("parent dir");

    let rust_gold_path = dir.join(format!("{}.roundtrip.rs", stem));
    let cpp_gold_path = dir.join(format!("{}.roundtrip.cpp", stem));

    // 1. Compile to WASM
    // We use a temporary file that is automatically deleted when it goes out of scope.
    let temp_wasm = tempfile::NamedTempFile::new().expect("Failed to create temp wasm file");
    let wasm_path = temp_wasm.path();

    let status = Command::new("rustc")
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "-O",
            "--crate-type=cdylib",
            "-o",
            wasm_path.to_str().expect("utf8 path"),
            source_path.to_str().expect("utf8 source path"),
        ])
        .output()
        .expect("Failed to execute rustc");

    if !status.status.success() {
        panic!(
            "rustc failed for {}\nstdout: {}\nstderr: {}",
            source_path_str,
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        );
    }

    // 2. Read WASM
    let wasm_bytes = fs::read(wasm_path).expect("Failed to read generated wasm");
    let bump = Bump::new();
    let mut reader = BinaryReader::new(&bump, wasm_bytes);
    let mut module = reader.parse_module().expect("Failed to parse WASM module");

    // 3. Decompile
    let mut lifter = Lifter::new();
    lifter.run(&mut module);

    let rust_out = RustPrinter::new(&module).print();
    let cpp_out = CPrinter::new(&module).print();

    // 4. Compare
    compare_output(&rust_out, &rust_gold_path, "Rust");
    compare_output(&cpp_out, &cpp_gold_path, "C++");
}

fn hex_to_path(path_str: &str) -> PathBuf {
    PathBuf::from(path_str)
}

fn compare_output(actual: &str, gold_path: &Path, lang: &str) {
    if !gold_path.exists() {
        println!(
            "--- ACTUAL OUTPUT ({}) ---\n{}\n--- END ACTUAL ---",
            lang, actual
        );
        return;
    }

    let expected = fs::read_to_string(gold_path).expect("Failed to read gold file");

    let actual_trimmed = actual.trim().replace("\r\n", "\n");
    let expected_trimmed = expected.trim().replace("\r\n", "\n");

    if actual_trimmed != expected_trimmed {
        let diff = TextDiff::from_lines(&expected_trimmed, &actual_trimmed);
        let mut diff_str = String::new();
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            diff_str.push_str(&format!("{}{}", sign, change));
        }
        panic!(
            "Mismatch in {} output for {:?}\nDIFF:\n{}",
            lang, gold_path, diff_str
        );
    }
}
