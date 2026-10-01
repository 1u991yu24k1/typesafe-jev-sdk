//! Deterministic allocation counts complement the noisy timing microbenchmark.
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use typesafe_jev_sdk::Question;
use typesafe_jev_sdk::builder::JevRequestBuilder;

struct CountingAllocator;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every allocation operation is forwarded unchanged to System;
// the extra bookkeeping uses allocation-free atomic operations.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() {
    for custom_model in [false, true] {
        // Warm up any one-time runtime initialization outside the count.
        for sample in 0..2 {
            ALLOCATIONS.store(0, Ordering::Relaxed);
            let mut builder = JevRequestBuilder::new();
            if custom_model {
                builder = builder.model(black_box("custom-model"));
            }
            let request = builder
                .state(black_box("state"))
                .question(
                    "ready",
                    Question::Noul {
                        instructions: "Ready?".to_owned(),
                        criteria: None,
                    },
                )
                .build()
                .unwrap();
            black_box(request);
            let allocations = ALLOCATIONS.load(Ordering::Relaxed);
            if sample == 1 {
                println!("builder custom_model={custom_model}: {allocations} allocation calls");
            }
        }
    }
}
