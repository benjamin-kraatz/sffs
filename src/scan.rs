use crate::cli::Args;
use crate::walker::{walk_parallel, WalkerStats};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub duration: Duration,
    pub top_files: Vec<(u64, PathBuf)>,
}

pub fn collect_scan_summary(args: &Args) -> ScanSummary {
    let stats = WalkerStats::new(args.top.is_some());
    let start_time = Instant::now();

    walk_parallel(args, &stats);

    let duration = start_time.elapsed();
    let total_size = stats.total_size.load(Ordering::SeqCst);
    let total_files = stats.total_files.load(Ordering::SeqCst);
    let total_dirs = stats.total_dirs.load(Ordering::SeqCst);

    let top_files = if let (Some(n), Some(top_mutex)) = (args.top, stats.top_files) {
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

        final_heap
            .into_sorted_vec()
            .into_iter()
            .map(|Reverse((size, path))| (size, path))
            .collect()
    } else {
        Vec::new()
    };

    ScanSummary {
        total_size,
        total_files,
        total_dirs,
        duration,
        top_files,
    }
}