use clap::Parser;
use ignore::WalkBuilder;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use mimalloc::MiMalloc;
use owo_colors::OwoColorize;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
#[command(author, version, about = "Super Fast File Size (sffs)", long_about = None)]
struct Args {
    /// Path(s) to check size for. If omitted, checks the current directory.
    #[arg()]
    paths: Vec<PathBuf>,

    /// Follow symbolic links
    #[arg(short = 'L', long)]
    follow_links: bool,

    /// Respect .gitignore files
    #[arg(short = 'g', long)]
    git_ignore: bool,

    /// Respect .ignore files
    #[arg(short = 'i', long)]
    ignore_files: bool,

    /// Ignore hidden files
    #[arg(short = 'H', long)]
    ignore_hidden: bool,

    /// Maximum depth to recurse
    #[arg(short = 'd', long)]
    max_depth: Option<usize>,

    /// Use the provided number of threads
    #[arg(short = 't', long)]
    threads: Option<usize>,

    /// Show size in raw bytes
    #[arg(short = 'b', long)]
    bytes: bool,

    /// Use SI units (1000 bytes = 1 KB) instead of 1024
    #[arg(long)]
    si: bool,

    /// Don't cross filesystem boundaries
    #[arg(short = 'x', long)]
    one_file_system: bool,

    /// Show top N largest files
    #[arg(long, value_name = "N")]
    top: Option<usize>,

    /// Suppress headers and footer
    #[arg(short = 's', long)]
    silent: bool,
}

fn apply_gradient(s: &str, start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8)) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n <= 1 {
        return s.truecolor(start_rgb.0, start_rgb.1, start_rgb.2).to_string();
    }

    let mut result = String::with_capacity(s.len() * 20);
    for (i, &c) in chars.iter().enumerate() {
        let t = i as f32 / (n - 1) as f32;
        let r = (start_rgb.0 as f32 * (1.0 - t) + end_rgb.0 as f32 * t) as u8;
        let g = (start_rgb.1 as f32 * (1.0 - t) + end_rgb.1 as f32 * t) as u8;
        let b = (start_rgb.2 as f32 * (1.0 - t) + end_rgb.2 as f32 * t) as u8;
        result.push_str(&c.truecolor(r, g, b).to_string());
    }
    result
}

fn draw_gradient_bar(width: usize, percentage: f64, start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8)) -> String {
    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let mut result = String::with_capacity(width * 20 + 8);
    result.push('▕');
    for i in 0..width {
        if i < filled {
            let t = i as f32 / (width.max(1) - 1).max(1) as f32;
            let r = (start_rgb.0 as f32 * (1.0 - t) + end_rgb.0 as f32 * t) as u8;
            let g = (start_rgb.1 as f32 * (1.0 - t) + end_rgb.1 as f32 * t) as u8;
            let b = (start_rgb.2 as f32 * (1.0 - t) + end_rgb.2 as f32 * t) as u8;
            result.push_str(&"█".truecolor(r, g, b).to_string());
        } else {
            result.push(' ');
        }
    }
    result.push('▏');
    result
}

fn format_size(bytes: u64, use_si: bool) -> String {
    let divisor = if use_si { 1000.0 } else { 1024.0 };
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= divisor && unit_idx < units.len() - 1 {
        size /= divisor;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, units[unit_idx])
    } else {
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, units[unit_idx])
        } else {
            format!("{:.2} {}", size, units[unit_idx])
        }
    }
}

fn main() {
    let mut args = Args::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Default to current directory if no paths provided
    if args.paths.is_empty() {
        args.paths.push(PathBuf::from("."));
    }

    let total_size = AtomicU64::new(0);
    let total_files = AtomicU64::new(0);
    let total_dirs = AtomicU64::new(0);

    let top_files = if args.top.is_some() {
        Some(Mutex::new(Vec::new()))
    } else {
        None
    };

    for path in &args.paths {
        if !path.exists() {
            eprintln!("Error: Path '{}' does not exist", path.display());
            continue;
        }

        // Handle direct files explicitly
        if path.is_file() {
            if let Ok(metadata) = path.metadata() {
                let s = metadata.len();
                total_size.fetch_add(s, Ordering::Relaxed);
                total_files.fetch_add(1, Ordering::Relaxed);
                if let Some(ref top_mutex) = top_files {
                    let mut heap = BinaryHeap::new();
                    let abs_p = if path.is_absolute() { path.clone() } else { cwd.join(path) };
                    heap.push(Reverse((s, abs_p)));
                    top_mutex.lock().unwrap().push(heap);
                }
            }
            continue;
        }

        // Handle directory traversal in parallel
        let abs_root = if path.is_absolute() { path.clone() } else { cwd.join(path) };
        let mut builder = WalkBuilder::new(abs_root);
        builder
            .follow_links(args.follow_links)
            .hidden(args.ignore_hidden)
            .git_ignore(args.git_ignore)
            .ignore(args.ignore_files)
            .max_depth(args.max_depth)
            .same_file_system(args.one_file_system);
            
        if let Some(threads) = args.threads {
            builder.threads(threads);
        }

        let size_ref = &total_size;
        let files_ref = &total_files;
        let dirs_ref = &total_dirs;
        let top_ref = &top_files;
        let n_top = args.top;

        builder.build_parallel().run(|| {
            let local_heap = n_top.map(|n| BinaryHeap::with_capacity(n + 1));
            
            struct ThreadLocalData<'a> {
                local_size: u64,
                local_files: u64,
                local_dirs: u64,
                heap: Option<BinaryHeap<Reverse<(u64, PathBuf)>>>,
                size_ref: &'a AtomicU64,
                files_ref: &'a AtomicU64,
                dirs_ref: &'a AtomicU64,
                top_ref: &'a Option<Mutex<Vec<BinaryHeap<Reverse<(u64, PathBuf)>>>>>,
            }
            impl<'a> Drop for ThreadLocalData<'a> {
                fn drop(&mut self) {
                    if self.local_size > 0 {
                        self.size_ref.fetch_add(self.local_size, Ordering::Relaxed);
                    }
                    if self.local_files > 0 {
                        self.files_ref.fetch_add(self.local_files, Ordering::Relaxed);
                    }
                    if self.local_dirs > 0 {
                        self.dirs_ref.fetch_add(self.local_dirs, Ordering::Relaxed);
                    }
                    if let Some(h) = self.heap.take() {
                        if !h.is_empty() {
                            if let Some(m) = self.top_ref {
                                m.lock().unwrap().push(h);
                            }
                        }
                    }
                }
            }
            
            let mut tld = ThreadLocalData {
                local_size: 0,
                local_files: 0,
                local_dirs: 0,
                heap: local_heap,
                size_ref,
                files_ref,
                dirs_ref,
                top_ref,
            };

            Box::new(move |result: Result<ignore::DirEntry, ignore::Error>| {
                if let Ok(entry) = result {
                    if let Ok(metadata) = entry.metadata() {
                        let s = metadata.len();
                        tld.local_size += s;
                        if metadata.is_dir() {
                            tld.local_dirs += 1;
                        } else {
                            tld.local_files += 1;
                        }

                        if let Some(ref mut heap) = tld.heap {
                            if metadata.is_file() {
                                let n = n_top.unwrap();
                                if heap.len() < n {
                                    heap.push(Reverse((s, entry.path().to_path_buf())));
                                } else if let Some(Reverse((min_s, _))) = heap.peek() {
                                    if s > *min_s {
                                        heap.pop();
                                        heap.push(Reverse((s, entry.path().to_path_buf())));
                                    }
                                }
                            }
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });
    }

    let final_size = total_size.load(Ordering::SeqCst);
    let final_files = total_files.load(Ordering::SeqCst);
    let final_dirs = total_dirs.load(Ordering::SeqCst);

    let size_str = if args.bytes {
        format!("{} B", final_size)
    } else {
        format_size(final_size, args.si)
    };

    if !args.silent {
        println!();
        let grad_size = apply_gradient(&size_str, (0, 255, 255), (255, 0, 255)); // Cyan to Magenta
        
        println!("  {}", "📊 SUMMARY".bold());
        println!("  {}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
        println!("    {:<12} ❯ {}", "Total Size".cyan().bold(), grad_size.bold());
        println!("    {:<12} ❯ {}", "Files".dimmed(), final_files.yellow());
        println!("    {:<12} ❯ {}", "Directories".dimmed(), final_dirs.blue());
        println!("  {}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
        println!();
    } else {
        println!("Total Size: {}", size_str);
    }

    if let (Some(n), Some(top_mutex)) = (args.top, top_files) {
        let heaps = top_mutex.into_inner().unwrap();
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
            println!("  {}", format!("🔥 TOP {} CONTRIBUTORS", n).bold());
            println!("  {}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
            println!("    {:<4} {:<12} {:<15} {}", "RANK".dimmed(), "SIZE".dimmed(), "IMPACT".dimmed(), "PATH".dimmed());

            let sorted_files: Vec<_> = final_heap.into_sorted_vec();
            
            // For the bar, we'll use the largest file in the top list as 100% 
            // to make the comparison between them visible, OR total size.
            // Let's use the largest file in the top list for better visual contrast.
            let max_top_size = sorted_files.get(0).map(|Reverse((s, _))| *s).unwrap_or(1) as f64;

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
                        format!("{}{}", "./".dimmed(), rel.display().bold())
                    }
                } else {
                    p.display().to_string().bold().to_string()
                };

                let relative_to_top = (*s as f64 / max_top_size) * 100.0;
                let bar = draw_gradient_bar(12, relative_to_top, (0, 255, 255), (255, 0, 255));
                
                let rank = format!("{:2}.", idx + 1);
                println!("    {:<4} {:<12} {:<15} {}", rank.dimmed(), s_str.green(), bar, p_display);
            }
            println!("  {}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
            println!();
        }
    }
}
