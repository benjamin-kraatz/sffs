use clap::Parser;
use mimalloc::MiMalloc;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use sffs::cli::Args;
use sffs::perf::{
    built_in_reference, format_speed_comparison, SpeedMetrics, BUILT_IN_REFERENCE_LABEL,
};
use sffs::render::{apply_gradient, draw_gradient_bar, format_size};
use sffs::scan::collect_scan_summary;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const SECTION_DIVIDER: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";

fn main() -> std::process::ExitCode {
    let mut args = Args::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Default to current directory if no paths provided
    if args.paths.is_empty() {
        args.paths.push(PathBuf::from("."));
    }

    let mut valid_paths = Vec::new();
    let mut exit_code = std::process::ExitCode::SUCCESS;
    for path in &args.paths {
        if path.exists() {
            valid_paths.push(path.clone());
        } else {
            eprintln!("Error: Path '{}' does not exist", path.display());
            exit_code = std::process::ExitCode::FAILURE;
        }
    }

    if valid_paths.is_empty() {
        return exit_code;
    }
    args.paths = valid_paths;

    let summary = collect_scan_summary(&args);

    let size_str = if args.bytes {
        format!("{} B", summary.total_size)
    } else {
        format_size(summary.total_size, args.si)
    };

    if !args.silent {
        println!();
        let grad_size = apply_gradient(&size_str, (0, 255, 255), (255, 0, 255)); // Cyan to Magenta

        println!("  {}", "📊 SUMMARY".bold());
        println!("  {}", SECTION_DIVIDER.dimmed());
        println!(
            "    {:<12} ❯ {}",
            "Total Size".cyan().bold(),
            grad_size.bold()
        );
        println!(
            "    {:<12} ❯ {}",
            "Files".dimmed(),
            summary.total_files.yellow()
        );
        println!(
            "    {:<12} ❯ {}",
            "Directories".dimmed(),
            summary.total_dirs.blue()
        );

        let speed_metrics = SpeedMetrics::from_summary(&summary);
        let total_ms = speed_metrics.total_ms;
        let speed_val = if total_ms < 1000.0 {
            format!("{:.1}ms", total_ms).bright_green().to_string()
        } else {
            format!("{:.2}s", total_ms / 1000.0)
                .bright_yellow()
                .to_string()
        };
        let per_file_val = speed_metrics
            .ms_per_file
            .map(|ms| format!("{ms:.3}ms/file").bright_blue().to_string())
            .unwrap_or_else(|| "n/a/file".dimmed().to_string());
        let benchmark_cmp = built_in_reference()
            .and_then(|reference| {
                speed_metrics.comparison_multiplier(reference.reference_entries_per_second)
            })
            .map(|multiplier| {
                let formatted = format_speed_comparison(multiplier);
                if multiplier >= 1.0 {
                    formatted.bright_magenta().to_string()
                } else {
                    formatted.dimmed().to_string()
                }
            })
            .unwrap_or_else(|| {
                format!("n/a vs {BUILT_IN_REFERENCE_LABEL}")
                    .dimmed()
                    .to_string()
            });
        let speed_str = format!("{} ({}, {})", speed_val, per_file_val, benchmark_cmp);
        println!("    {:<12} ❯ {}", "Speed".dimmed(), speed_str);

        println!("  {}", SECTION_DIVIDER.dimmed());
        println!();
    } else {
        println!("Total Size: {}", size_str);
    }

    if let Some(n) = args.top {
        if !summary.top_files.is_empty() {
            println!("  {}", format!("TOP {}", n).bold());
            println!("  {}", SECTION_DIVIDER.dimmed());
            println!(
                "    {:<4} {:<12} {:<15} {}",
                "RANK".dimmed(),
                "SIZE".dimmed(),
                "IMPACT".dimmed(),
                "PATH".dimmed()
            );

            let max_top_size = summary.top_files.first().map(|(s, _)| *s).unwrap_or(1) as f64;

            for (idx, (s, p)) in summary.top_files.iter().enumerate() {
                let s_str = if args.bytes {
                    format!("{} B", s)
                } else {
                    format_size(*s, args.si)
                };

                let p_display = if let Ok(rel) = p.strip_prefix(&cwd) {
                    if rel.as_os_str().is_empty() {
                        ".".bold().to_string()
                    } else {
                        format!("{}{}", "./".dimmed(), rel.display().to_string().bold())
                    }
                } else {
                    p.display().to_string().bold().to_string()
                };

                let relative_to_top = (*s as f64 / max_top_size) * 100.0;
                let bar = draw_gradient_bar(12, relative_to_top, (0, 255, 255), (255, 0, 255));

                let rank = format!("{:2}.", idx + 1);
                println!(
                    "    {:<4} {:<12} {:<15} {}",
                    rank.dimmed(),
                    s_str.green(),
                    bar,
                    p_display
                );
            }
            println!("  {}", SECTION_DIVIDER.dimmed());
            println!();
        }
    }
    exit_code
}
