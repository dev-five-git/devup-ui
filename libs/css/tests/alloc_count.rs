//! Dependency-free allocation-count harness for `optimize_value()`.
//!
//! Uses a counting global allocator (a thin `System` wrapper that increments an
//! `AtomicUsize` on every `alloc`/`realloc`) — no runtime dependency, no
//! library-code change. It measures the allocation COUNT around a fixed,
//! representative batch of `optimize_value` calls so the memory profile of the
//! single hottest CSS helper (called for every static value and every theme
//! color/length/shadow token) has a deterministic regression guard, independent
//! of noisy wall-clock benchmarks.
//!
//! `optimize_value` had no allocation-count guard while `sheet` did
//! (`to_css_allocation_count_is_bounded`); this mirrors that harness. Kept as its
//! own integration test so the counting allocator only ever affects THIS test
//! binary and never production builds.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use css::optimize_value::optimize_value;

struct CountingAlloc;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
/// When non-zero, `alloc`/`realloc` calls are tallied into `ALLOC_COUNT`. Kept
/// off outside the measured region so unrelated allocations (test harness,
/// setup, regex warm-up) are not counted.
static COUNTING: AtomicUsize = AtomicUsize::new(0);

// SAFETY: `CountingAlloc` forwards every call verbatim to the platform `System`
// allocator and only performs a relaxed atomic increment as a side effect, so it
// upholds all `GlobalAlloc` invariants exactly as `System` does.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) != 0 {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: forwarding an unchanged `layout` to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarding the exact `ptr`/`layout` pair we handed out.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) != 0 {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: forwarding the exact `ptr`/`layout`/`new_size` triple.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// A fixed, representative slice of inputs: a mix of common no-op / cheap values
/// (`red`, `14px`, `$primary`, `0px`) and heavier values that exercise the
/// regex-replacement paths (rgba/rgb collapse, shadow lists, math functions).
const INPUTS: &[&str] = &[
    "red",
    "14px",
    "$primary",
    "0px",
    "rgba(255, 0, 0, 0.5)",
    "0px 0px 10px rgba(0,0,0,0.1)",
    "clamp(0, 10px, 10px)",
    "translate(10px, 0px)",
    "#FF0000",
    "10px 0",
];

/// Count allocations performed by one pass of `optimize_value` over `INPUTS`.
fn measure_optimize_value_allocs() -> usize {
    // Warm any lazily-initialised statics (the `LazyLock` regexes inside
    // `optimize_value`) OUTSIDE the counted region so their one-time
    // initialisation is not charged to this measurement.
    for input in INPUTS {
        let _ = optimize_value(input);
    }

    ALLOC_COUNT.store(0, Ordering::Relaxed);
    COUNTING.store(1, Ordering::Relaxed);
    let mut sink = 0usize;
    for input in INPUTS {
        let out = optimize_value(input);
        // Keep each result observably alive so no call can be optimised away.
        sink = sink.wrapping_add(out.len());
    }
    COUNTING.store(0, Ordering::Relaxed);
    let count = ALLOC_COUNT.load(Ordering::Relaxed);

    assert!(sink > 0, "optimize_value must produce non-empty output");
    count
}

#[test]
fn optimize_value_allocation_count_is_bounded() {
    // Deterministic allocation ceiling for one `optimize_value` pass over the
    // fixed `INPUTS` batch. This harness pins the allocation COUNT so that any
    // future change which reintroduces a throwaway `String`/regex-owned buffer on
    // the common no-op path (e.g. `red`, `14px`, `0px`) — or an extra
    // intermediate allocation on the heavier rgba/shadow/math paths — trips this
    // test instead of silently regressing memory use.
    //
    // The measured count for the current implementation is 90 allocations over
    // this 10-input batch (the heavier rgba/rgb collapse, shadow-list and math
    // paths each allocate intermediate `String`s via `format!` inside their regex
    // replacement closures). The bound below leaves ~20% headroom (so unrelated,
    // legitimate allocation changes do not make the test brittle) while still
    // catching a reintroduction of per-value heap churn on the common no-op path,
    // which would push the count well past this ceiling.
    const MAX_ALLOCS: usize = 108;

    let allocs = measure_optimize_value_allocs();

    assert!(
        allocs <= MAX_ALLOCS,
        "optimize_value allocated {allocs} times over {} inputs; \
         expected <= {MAX_ALLOCS} (regression: redundant intermediate allocations reintroduced?)",
        INPUTS.len()
    );
}
