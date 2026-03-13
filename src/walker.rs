use ignore::{DirEntry, WalkBuilder, WalkState};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::cli::Args;

pub type FileHeap = BinaryHeap<Reverse<(u64, PathBuf)>>;
pub type TopFiles = Mutex<Vec<FileHeap>>;

const PARALLEL_ROOT_ENTRY_HINT_THRESHOLD: usize = 32;
const PARALLEL_ROOT_PATH_HINT_THRESHOLD: usize = 4;

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
            self.stats
                .total_size
                .fetch_add(self.local_size, Ordering::Relaxed);
        }
        if self.local_files > 0 {
            self.stats
                .total_files
                .fetch_add(self.local_files, Ordering::Relaxed);
        }
        if self.local_dirs > 0 {
            self.stats
                .total_dirs
                .fetch_add(self.local_dirs, Ordering::Relaxed);
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

    if should_walk_serial(args) {
        walk_serial(args, stats);
    } else {
        walk_parallel_inner(args, stats);
    }
}

fn walk_parallel_inner(args: &Args, stats: &WalkerStats) {
    let mut builder = configured_builder(args);

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
            process_entry_result(result, &mut tld);
            WalkState::Continue
        })
    });
}

fn walk_serial(args: &Args, stats: &WalkerStats) {
    let mut tld = ThreadLocalData {
        local_size: 0,
        local_files: 0,
        local_dirs: 0,
        heap: args.top.map(|n| BinaryHeap::with_capacity(n + 1)),
        stats,
        n_top: args.top,
    };

    for result in configured_builder(args).build() {
        process_entry_result(result, &mut tld);
    }
}

fn configured_builder(args: &Args) -> WalkBuilder {
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

    builder
}

fn process_entry_result(result: Result<DirEntry, ignore::Error>, tld: &mut ThreadLocalData<'_>) {
    if let Ok(entry) = result {
        if entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false)
        {
            tld.local_dirs += 1;
            return;
        }

        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                tld.local_dirs += 1;
                return;
            }

            let size = metadata.len();
            tld.local_size += size;
            tld.local_files += 1;
            if let Some(ref mut heap) = tld.heap {
                let n = tld.n_top.unwrap();
                if heap.len() < n {
                    heap.push(Reverse((size, entry.path().to_path_buf())));
                } else if let Some(Reverse((min_size, _))) = heap.peek() {
                    if size > *min_size {
                        heap.pop();
                        heap.push(Reverse((size, entry.path().to_path_buf())));
                    }
                }
            }
        }
    }
}

fn should_walk_serial(args: &Args) -> bool {
    match args.threads {
        Some(threads) => threads <= 1,
        None => !has_parallelism_hint(args),
    }
}

fn has_parallelism_hint(args: &Args) -> bool {
    if args.paths.len() >= PARALLEL_ROOT_PATH_HINT_THRESHOLD {
        return true;
    }

    let mut hinted_entries = 0usize;
    for path in &args.paths {
        hinted_entries += estimate_root_entries(path, args);
        if hinted_entries >= PARALLEL_ROOT_ENTRY_HINT_THRESHOLD {
            return true;
        }
    }

    false
}

fn estimate_root_entries(path: &std::path::Path, args: &Args) -> usize {
    let metadata = if args.follow_links {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    };

    let Ok(metadata) = metadata else {
        return PARALLEL_ROOT_ENTRY_HINT_THRESHOLD;
    };

    if !metadata.is_dir() {
        return 1;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return PARALLEL_ROOT_ENTRY_HINT_THRESHOLD;
    };

    let mut total = 1usize;
    for entry in entries.flatten() {
        if args.ignore_hidden && is_hidden(&entry) {
            continue;
        }
        total += 1;
        if total >= PARALLEL_ROOT_ENTRY_HINT_THRESHOLD {
            return total;
        }
    }

    total
}

fn is_hidden(entry: &fs::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::{prepare_fixture, ScenarioKind};
    use std::path::PathBuf;

    fn base_args(path: PathBuf) -> Args {
        Args {
            paths: vec![path],
            follow_links: false,
            git_ignore: false,
            ignore_files: false,
            ignore_hidden: false,
            max_depth: None,
            threads: None,
            bytes: true,
            si: false,
            one_file_system: false,
            top: None,
            silent: true,
        }
    }

    #[test]
    fn adaptive_mode_prefers_serial_for_deep_tree_fixture() {
        let fixture = prepare_fixture(ScenarioKind::DeepTree).unwrap();
        let args = base_args(fixture.path().to_path_buf());

        assert!(should_walk_serial(&args));
    }

    #[test]
    fn adaptive_mode_prefers_parallel_for_tiny_files_fixture() {
        let fixture = prepare_fixture(ScenarioKind::TinyFiles).unwrap();
        let args = base_args(fixture.path().to_path_buf());

        assert!(!should_walk_serial(&args));
    }

    #[test]
    fn default_mode_prefers_parallel_for_wide_tree_fixture() {
        let fixture = prepare_fixture(ScenarioKind::WideTree).unwrap();
        let args = base_args(fixture.path().to_path_buf());

        assert!(!should_walk_serial(&args));
    }

    #[test]
    fn default_mode_prefers_serial_for_mixed_tree_fixture() {
        let fixture = prepare_fixture(ScenarioKind::MixedTree).unwrap();
        let args = base_args(fixture.path().to_path_buf());

        assert!(should_walk_serial(&args));
    }

    #[test]
    fn explicit_thread_override_preserves_parallel_choice() {
        let fixture = prepare_fixture(ScenarioKind::DeepTree).unwrap();
        let mut args = base_args(fixture.path().to_path_buf());
        args.threads = Some(4);

        assert!(!should_walk_serial(&args));
    }

    #[test]
    fn explicit_single_thread_override_forces_serial() {
        let fixture = prepare_fixture(ScenarioKind::TinyFiles).unwrap();
        let mut args = base_args(fixture.path().to_path_buf());
        args.threads = Some(1);

        assert!(should_walk_serial(&args));
    }
}
