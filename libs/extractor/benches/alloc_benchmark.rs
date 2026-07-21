//! Dependency-free allocation-counting harness for the extractor hot path.
//!
//! This is a SEPARATE `[[bench]]` target so its `#[global_allocator]` never
//! perturbs the criterion timing bench (`extract_benchmark`). It installs a
//! counting `GlobalAlloc` wrapper that delegates to `std::alloc::System` and
//! tallies allocation COUNT and BYTES, then snapshots those counters around a
//! single `extract(...)` call for SMALL/MEDIUM/LARGE inputs.
//!
//! It gives every future memory optimization an attributable before/after
//! number and locks in the allocation-count reductions already made. Run with:
//!
//! ```bash
//! cargo bench -p extractor --bench alloc_benchmark
//! ```
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use css::class_map::reset_class_map;
use css::debug::set_debug;
use css::file_map::reset_file_map;
use css::set_prefix;
use extractor::{ExtractOption, extract};

/// A counting allocator that forwards every request to the system allocator
/// while tracking the total number of allocations and bytes requested.
///
/// Only `alloc`/`alloc_zeroed`/`realloc` bump the counters (deallocations do
/// not, since we measure allocation pressure, not live heap). Counters use
/// `Relaxed` ordering: the bench is single-threaded around each snapshot, so no
/// cross-thread synchronization of the counter values is required.
struct CountingAlloc {
    allocations: AtomicUsize,
    bytes: AtomicUsize,
}

impl CountingAlloc {
    const fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            bytes: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self) -> (usize, usize) {
        (
            self.allocations.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
        )
    }
}

// SAFETY: every method forwards directly to `System`, which is a valid
// `GlobalAlloc`. The counter updates are plain atomic stores that never touch
// the returned pointers, so they cannot violate the allocator contract.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTER: CountingAlloc = CountingAlloc::new();

fn make_option() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "@devup-ui/react".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::new(),
    }
}

fn reset_state() {
    reset_class_map();
    reset_file_map();
    set_debug(false);
    set_prefix(None);
}

const SMALL_INPUT: &str = r#"import {Box} from '@devup-ui/react'
const a = <Box bg="red" p={4} />"#;

const MEDIUM_INPUT: &str = r#"import {Box, Flex, Text} from '@devup-ui/react'
const a = <Flex gap={2} direction="column">
  <Box bg="red" p={4} m={2} _hover={{bg: "blue"}} borderRadius="8px" />
  <Box bg={["red", "blue", "green"]} p={[1,2,3]} />
  <Text color="white" fontSize="14px" fontWeight="bold" _focus={{color: "red"}} />
  <Box display="flex" alignItems="center" justifyContent="center" w="100%" h="50vh" />
  <Box border="solid 1px red" boxShadow="0 4px 6px rgba(0,0,0,0.1)" transition="all 0.3s" />
</Flex>"#;

const LARGE_INPUT: &str = r#"import {Box, Flex, Text, Grid} from '@devup-ui/react'
const a = <Flex direction="column" gap={4}>
  <Box bg="red" color="white" p={4} m={2} w="100%" h="auto" />
  <Box bg={["red", "blue", "green"]} p={[1,2,3]} m={[0,1,2]} />
  <Box _hover={{bg: "blue", color: "red"}} _focus={{outline: "none"}} _active={{bg: "darkblue"}} bg="gray" />
  <Text fontSize={["12px", "14px", "16px"]} fontWeight="bold" letterSpacing="0.5px" lineHeight="1.5" color="$primary" />
  <Grid templateColumns="repeat(3, 1fr)" gap={4} p={2} bg="white" borderRadius="8px" />
  <Box display="flex" alignItems="center" justifyContent="center" flexDirection="row" flexWrap="wrap" />
  <Box border="solid 1px red" borderRadius="4px" boxShadow="0 4px 6px rgba(0,0,0,0.1)" />
  <Box transition="all 0.3s ease-in-out" transform="translateX(0px)" opacity="1" />
  <Box position="absolute" top="0" left="0" right="0" bottom="0" zIndex="10" />
  <Box overflow="hidden" textOverflow="ellipsis" whiteSpace="nowrap" maxW="200px" />
  <Box bg="$primary" color="$text" p={4} borderColor="$primary" />
  <Flex gap={2} alignItems="center" justifyContent="space-between" w="100%" />
  <Box _hover={{transform: "scale(1.05)", boxShadow: "0 8px 16px rgba(0,0,0,0.2)"}} cursor="pointer" />
  <Text textAlign="center" textTransform="uppercase" textDecoration="none" fontFamily="sans-serif" />
  <Box minW="0" minH="0" maxW="100%" maxH="100vh" flex="1" />
  <Grid templateRows="auto 1fr auto" minH="100vh" bg="white" />
  <Box backgroundImage="linear-gradient(to right, red, blue)" backgroundSize="cover" />
  <Box _before={{content: "''", display: "block", w: "100%", h: "2px", bg: "red"}} />
  <Box animation="spin 1s linear infinite" />
  <Box userSelect="none" pointerEvents="auto" visibility="visible" />
  <Box borderTop="1px solid gray" borderBottom="1px solid gray" px={4} py={2} />
  <Box gap={[2, 4, 6]} rowGap={2} columnGap={4} />
  <Box w={["100%", "50%", "33%"]} h={["auto", "200px", "300px"]} />
  <Box _hover={{_before: {bg: "blue"}}} p={4} />
  <Box objectFit="cover" objectPosition="center" aspectRatio="16/9" />
  <Flex direction={["column", "row"]} wrap="wrap" gap={[2, 4]} />
  <Box outlineColor="blue" outlineWidth="2px" outlineStyle="solid" outlineOffset="2px" />
  <Box backdropFilter="blur(10px)" filter="brightness(0.9)" />
  <Box scrollBehavior="smooth" overscrollBehavior="contain" />
  <Box placeSelf="center" placeItems="center" placeContent="center" />
</Flex>"#;

/// Measure allocation count + bytes for one `extract` of `input`.
///
/// `reset_state()` runs BEFORE the snapshot so its own allocations are excluded
/// from the measured window.
fn measure(label: &str, filename: &str, input: &str) {
    reset_state();
    let (a0, b0) = COUNTER.snapshot();
    let result = extract(black_box(filename), black_box(input), make_option()).unwrap();
    black_box(&result);
    let (a1, b1) = COUNTER.snapshot();
    println!(
        "alloc_bench {label:<7} allocations={:>7} bytes={:>9}",
        a1 - a0,
        b1 - b0
    );
}

fn main() {
    // Warm any lazily-initialized statics so their one-time allocations are not
    // attributed to the first measured input.
    reset_state();
    let _ = extract("warmup.tsx", SMALL_INPUT, make_option()).unwrap();

    measure("SMALL", "small.tsx", SMALL_INPUT);
    measure("MEDIUM", "medium.tsx", MEDIUM_INPUT);
    measure("LARGE", "large.tsx", LARGE_INPUT);
}
