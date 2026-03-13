use sffs::benchmark::{artifact_markdown_table, generate_reference_artifact, BenchmarkConfig};
use std::fs;
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("benchmark reference generation failed: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = PathBuf::from("docs/benchmarks/reference.json");
    let mut config = BenchmarkConfig::default();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let Some(path) = args.next() else {
                    return Err("missing path after --output".into());
                };
                output = PathBuf::from(path);
            }
            "--iterations" => {
                let Some(value) = args.next() else {
                    return Err("missing value after --iterations".into());
                };
                config.measurement_iterations = value.parse()?;
            }
            "--warmup" => {
                let Some(value) = args.next() else {
                    return Err("missing value after --warmup".into());
                };
                config.warmup_iterations = value.parse()?;
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }

    let artifact = generate_reference_artifact(config)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_string_pretty(&artifact)?)?;

    println!("Wrote {}", output.display());
    println!();
    println!("{}", artifact_markdown_table(&artifact));

    Ok(())
}