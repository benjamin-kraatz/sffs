use ignore::WalkBuilder;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::cli::Args;

pub type FileHeap = BinaryHeap<Reverse<(u64, PathBuf)>>;
pub type TopFiles = Mutex<Vec<FileHeap>>;

pub struct WalkerStats {
    pub total_size: AtomicU64,
    pub total_files: AtomicU64,
    pub total_dirs: AtomicU64,
    pub top_files: Option<TopFiles>,
}

impl WalkerStats {
    pub fn new(show_top: bool) -> Self {
        Self {
            total_size: AtomicU64::new(0),
            total_files: AtomicU64::new(0),
            total_dirs: AtomicU64::new(0),
            top_files: if show_top {
                Some(Mutex::new(Vec::new()))
            } else {
                None
            },
        }
    }
}

struct ThreadLocalData<'a> {
    local_size: u64,
    local_files: u64,
    local_dirs: u64,
    heap: Option<FileHeap>,
    stats: &'a WalkerStats,
    n_top: Option<usize>,
}

impl<'a> Drop for ThreadLocalData<'a> {
    fn drop(&mut self) {
        if self.local_size > 0 {
            self.stats.total_size.fetch_add(self.local_size, Ordering::Relaxed);
        }
        if self.local_files > 0 {
            self.stats.total_files.fetch_add(self.local_files, Ordering::Relaxed);
        }
        if self.local_dirs > 0 {
            self.stats.total_dirs.fetch_add(self.local_dirs, Ordering::Relaxed);
        }
        if let Some(h) = self.heap.take() {
            if !h.is_empty() {
                if let Some(ref m) = self.stats.top_files {
                    let mut guard = m.lock().unwrap_or_else(|e| e.into_inner());
                    guard.push(h);
                }
            }
        }
    }
}

pub fn walk_parallel(args: &Args, stats: &WalkerStats) {
    if args.paths.is_empty() {
        return;
    }

    let mut builder = WalkBuilder::new(&args.paths[0]);
    for path in &args.paths[1..] {
        builder.add(path);
    }

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

    let n_top = args.top;

    builder.build_parallel().run(|| {
        let local_heap = n_top.map(|n| BinaryHeap::with_capacity(n + 1));
        let mut tld = ThreadLocalData {
            local_size: 0,
            local_files: 0,
            local_dirs: 0,
            heap: local_heap,
            stats,
            n_top,
        };

        Box::new(move |result| {
            if let Ok(entry) = result {
                if let Ok(metadata) = entry.metadata() {
                    let s = metadata.len();
                    tld.local_size += s;
                    if metadata.is_dir() {
                        tld.local_dirs += 1;
                    } else {
                        tld.local_files += 1;
                    }

                    if let (Some(ref mut heap), true) = (&mut tld.heap, metadata.is_file()) {
                        let n = tld.n_top.unwrap();
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
            ignore::WalkState::Continue
        })
    });
}
