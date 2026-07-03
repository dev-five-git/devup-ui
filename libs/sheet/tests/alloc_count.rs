//! Dependency-free allocation-count harness for `Theme::to_css()`.
//!
//! Uses a counting global allocator (a thin `System` wrapper that increments an
//! `AtomicUsize` on every `alloc`) — no runtime dependency, no library-code change.
//! It measures the allocation COUNT around a single `to_css()` call on a large theme
//! so the memory win from reusing precomputed default-variant optimized colors is
//! measurable and deterministic (independent of noisy wall-clock benchmarks).
//!
//! Kept as its own integration test so the counting allocator only ever affects THIS
//! test binary and never production builds.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use sheet::theme::{ColorTheme, Theme};

struct CountingAlloc;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
/// When true, `alloc` calls are tallied into `ALLOC_COUNT`. Kept off outside the
/// measured region so unrelated allocations (test harness, setup) are not counted.
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

const COLOR_COUNT: usize = 80;

fn make_large_color_theme() -> Theme {
    let mut theme = Theme::default();
    let mut default_colors = ColorTheme::default();
    let mut dark_colors = ColorTheme::default();

    for idx in 0..COLOR_COUNT {
        let name = format!("color.{idx}");
        default_colors.add_color(&name, &format!("#{idx:02x}{idx:02x}{idx:02x}"));
        dark_colors.add_color(
            &name,
            &format!("#{:02x}{:02x}{:02x}", 255 - idx, 255 - idx, 255 - idx),
        );
    }
    theme.add_color_theme("default", default_colors);
    theme.add_color_theme("dark", dark_colors);
    theme
}

/// Count allocations performed by exactly one `to_css()` call.
fn measure_to_css_allocs(theme: &Theme) -> usize {
    // Warm any lazily-initialised statics (e.g. regexes inside `optimize_value`)
    // OUTSIDE the counted region so they are not charged to this measurement.
    let _ = theme.to_css();

    ALLOC_COUNT.store(0, Ordering::Relaxed);
    COUNTING.store(1, Ordering::Relaxed);
    let css = theme.to_css();
    COUNTING.store(0, Ordering::Relaxed);
    let count = ALLOC_COUNT.load(Ordering::Relaxed);

    // Keep `css` observably alive so the whole call cannot be optimised away.
    assert!(!css.is_empty(), "to_css must produce output");
    count
}

#[test]
fn to_css_allocation_count_is_bounded() {
    // Deterministic allocation ceiling for `to_css()` on an 80-default + 80-dark
    // color theme. This harness pins the allocation COUNT so that any future change
    // which reintroduces per-color heap churn (e.g. allocating a fresh `String` for
    // every default-variant color instead of reusing the precomputed, `Cow`-borrowed
    // optimized value) trips this test instead of silently regressing memory use.
    //
    // The measured count for the current implementation is ~2027 allocations. The
    // bound below leaves generous headroom (so unrelated, legitimate allocation
    // changes elsewhere do not make the test brittle) while still catching a
    // reintroduction of ~80+ per-color allocations, which would push the count well
    // past this ceiling.
    const MAX_ALLOCS: usize = 2500;

    let theme = make_large_color_theme();
    let allocs = measure_to_css_allocs(&theme);

    assert!(
        allocs <= MAX_ALLOCS,
        "to_css allocated {allocs} times for a {COLOR_COUNT}-color theme; \
         expected <= {MAX_ALLOCS} (regression: redundant per-color heap allocations reintroduced?)"
    );
}
