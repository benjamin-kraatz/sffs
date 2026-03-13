mod cli;
mod render;
mod walker;

use clap::Parser;
use mimalloc::MiMalloc;
use owo_colors::OwoColorize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::cli::Args;
use crate::render::{apply_gradient, draw_gradient_bar, format_size};
use crate::walker::{walk_parallel, WalkerStats};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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

    let stats = WalkerStats::new(args.top.is_some());
    let start_time = Instant::now();

    walk_parallel(&args, &stats);

    let duration = start_time.elapsed();
    let final_size = stats.total_size.load(Ordering::SeqCst);
    let final_files = stats.total_files.load(Ordering::SeqCst);
    let final_dirs = stats.total_dirs.load(Ordering::SeqCst);

    let size_str = if args.bytes {
        format!("{} B", final_size)
    } else {
        format_size(final_size, args.si)
    };

    if !args.silent {
        println!();
        let grad_size = apply_gradient(&size_str, (0, 255, 255), (255, 0, 255)); // Cyan to Magenta

        println!("  {}", "📊 SUMMARY".bold());
        println!(
            "  {}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
        );
        println!(
            "    {:<12} ❯ {}",
            "Total Size".cyan().bold(),
            grad_size.bold()
        );
        println!("    {:<12} ❯ {}", "Files".dimmed(), final_files.yellow());
        println!("    {:<12} ❯ {}", "Directories".dimmed(), final_dirs.blue());

        let ms_per_file = if final_files > 0 {
            duration.as_secs_f64() * 1000.0 / final_files as f64
        } else {
            0.0
        };
        let total_ms = duration.as_secs_f64() * 1000.0;
        let speed_val = if total_ms < 1000.0 {
            format!("{:.1}ms", total_ms).bright_green().to_string()
        } else {
            format!("{:.2}s", total_ms / 1000.0)
                .bright_yellow()
                .to_string()
        };
        let per_file_val = format!("{:.3}ms/file", ms_per_file)
            .bright_blue()
            .to_string();
        let speed_str = format!("{} ({})", speed_val, per_file_val);
        println!("    {:<12} ❯ {}", "Speed".dimmed(), speed_str);

        println!(
            "  {}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
        );
        println!();
    } else {
        println!("Total Size: {}", size_str);
    }

    if let (Some(n), Some(top_mutex)) = (args.top, stats.top_files) {
        let heaps = top_mutex.into_inner().unwrap_or_else(|e| e.into_inner());
        let mut final_heap = BinaryHeap::with_capacity(n + 1);
        for heap in heaps {
            for item in heap {
                final_heap.push(item);
                if final_heap.len() > n {
                    final_heap.pop();
                }
            }
        }

        if !final_heap.is_empty() {
            println!("  {}", format!("TOP {}", n).bold());
            println!(
                "  {}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
            );
            println!(
                "    {:<4} {:<12} {:<15} {}",
                "RANK".dimmed(),
                "SIZE".dimmed(),
                "IMPACT".dimmed(),
                "PATH".dimmed()
            );

            let sorted_files: Vec<_> = final_heap.into_sorted_vec();
            let max_top_size = sorted_files.first().map(|Reverse((s, _))| *s).unwrap_or(1) as f64;

            for (idx, Reverse((s, p))) in sorted_files.iter().enumerate() {
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
            println!(
                "  {}",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
            );
            println!();
        }
    }
    exit_code
}
