use clap::Parser;
use ignore::WalkBuilder;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use mimalloc::MiMalloc;

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
    
    // Default to current directory if no paths provided
    if args.paths.is_empty() {
        args.paths.push(PathBuf::from("."));
    }

    let total_size = AtomicU64::new(0);
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
                if let Some(ref top_mutex) = top_files {
                    let mut heap = BinaryHeap::new();
                    heap.push(Reverse((s, path.clone())));
                    top_mutex.lock().unwrap().push(heap);
                }
            }
            continue;
        }

        // Handle directory traversal in parallel
        let mut builder = WalkBuilder::new(path);
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
        let top_ref = &top_files;
        let n_top = args.top;

        builder.build_parallel().run(|| {
            let local_heap = n_top.map(|n| BinaryHeap::with_capacity(n + 1));
            
            struct ThreadLocalData<'a> {
                local_size: u64,
                heap: Option<BinaryHeap<Reverse<(u64, PathBuf)>>>,
                size_ref: &'a AtomicU64,
                top_ref: &'a Option<Mutex<Vec<BinaryHeap<Reverse<(u64, PathBuf)>>>>>,
            }
            impl<'a> Drop for ThreadLocalData<'a> {
                fn drop(&mut self) {
                    if self.local_size > 0 {
                        self.size_ref.fetch_add(self.local_size, Ordering::Relaxed);
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
                heap: local_heap,
                size_ref,
                top_ref,
            };

            Box::new(move |result: Result<ignore::DirEntry, ignore::Error>| {
                if let Ok(entry) = result {
                    if let Ok(metadata) = entry.metadata() {
                        let s = metadata.len();
                        tld.local_size += s;
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
    if args.bytes {
        println!("# Total Size: {} B", final_size);
    } else {
        println!("# Total Size: {}", format_size(final_size, args.si));
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
            println!("\n# Top {} Largest Files:", n);
            let sorted_files: Vec<_> = final_heap.into_sorted_vec();
            for Reverse((s, p)) in sorted_files {
                let s_str = if args.bytes {
                    format!("{} B", s)
                } else {
                    format_size(s, args.si)
                };
                println!("{}: {}", s_str, p.display());
            }
        }
    }
}
