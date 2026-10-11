//! Isolated allocation-budget regressions: a regression exits only the child,
//! before requesting a dangerous allocation from the system allocator.
use ratex_layout::{layout, to_display_list, validate_layout, LayoutOptions};
use ratex_parser::parse;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

struct BudgetAllocator;
static ARMED: AtomicBool = AtomicBool::new(false);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
const BUDGET: usize = 64 * 1024 * 1024;
fn charge(size: usize) {
    if ARMED.load(Relaxed) && ALLOCATED.fetch_add(size, Relaxed).saturating_add(size) > BUDGET {
        std::process::exit(77);
    }
}
unsafe impl GlobalAlloc for BudgetAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        charge(size);
        System.realloc(ptr, layout, size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}
#[global_allocator]
static ALLOCATOR: BudgetAllocator = BudgetAllocator;

fn nested(prefix: &str, suffix: &str, n: usize) -> String {
    format!("{}x{}", prefix.repeat(n), suffix.repeat(n))
}
fn bounded_validated_layout(input: &str) -> (Result<ratex_layout::LayoutBox, String>, usize) {
    let ast = parse(input).unwrap();
    ALLOCATED.store(0, Relaxed);
    ARMED.store(true, Relaxed);
    let boxes = layout(&ast, &LayoutOptions::default());
    let result = validate_layout(&boxes).map(|()| boxes);
    ARMED.store(false, Relaxed);
    (result, ALLOCATED.load(Relaxed))
}
fn bounded_layout(input: &str) -> (ratex_layout::LayoutBox, usize) {
    let (result, bytes) = bounded_validated_layout(input);
    (result.unwrap(), bytes)
}
fn run_cases() {
    // Warm up one-time font and parser initialization outside the budget.
    bounded_layout("x");
    // Geometry-dependent dashed separators bypass the parser's AST budget.
    // Eight items per layer give an exact accepted DisplayList boundary.
    // Public layout trees remain owned; the allocator caps the full tree and
    // validation cost, while drawing-item expansion happens only after validation.
    let dashed = |height: &str, cols: &str, layers: usize| {
        format!(
            r"{}\begin{{array}}{{{cols}}}\rule{{1em}}{{{height}em}}\end{{array}}{}",
            r"\pmb{".repeat(layers),
            "}".repeat(layers)
        )
    };
    let boundary = dashed("1.6", ":c", 13);
    let (boxes, _) = bounded_layout(&boundary);
    ALLOCATED.store(0, Relaxed);
    ARMED.store(true, Relaxed);
    assert_eq!(
        to_display_list(&boxes).items.len(),
        ratex_layout::MAX_LAYOUT_OUTPUT_ITEMS
    );
    ARMED.store(false, Relaxed);
    // The original API still returns a complete DisplayList directly, even
    // above the validator's cap: no source break, panic, or truncation.
    let first_rejected = boundary + "x";
    let boxes = layout(&parse(&first_rejected).unwrap(), &LayoutOptions::default());
    ALLOCATED.store(0, Relaxed);
    ARMED.store(true, Relaxed);
    let legacy: ratex_types::display_item::DisplayList = ratex_layout::to_display_list(&boxes);
    assert_eq!(
        legacy.items.len(),
        ratex_layout::MAX_LAYOUT_OUTPUT_ITEMS + 1
    );
    drop(legacy);
    ARMED.store(false, Relaxed);
    for input in [
        first_rejected,          // First item beyond the validator's limit.
        dashed("10", ":c:", 13), // Reported 123-byte amplification.
        dashed("1000", ":c:", 13),
        dashed("1000000000", ":c", 0), // Height alone, without bold layers.
        dashed("10", ":c:", 9) + &dashed("10", ":c:", 9), // Sibling costs add up.
        format!(r"\scriptstyle {}", dashed("1000", ":c:", 13)),
        format!(r"\tiny {}", dashed("1000", ":c:", 13)),
    ] {
        let (result, bytes) = bounded_validated_layout(&input);
        assert_eq!(
            result.unwrap_err(),
            "Layout resource limit exceeded",
            "{input}"
        );
        assert!(bytes < BUDGET, "rejection allocated {bytes} bytes: {input}");
    }
    assert!(!to_display_list(&bounded_layout("x+1").0).items.is_empty());
    let input = nested(r"\pmb{", "}", 16);
    assert_eq!(input.len(), 97);
    assert!(parse(&input)
        .unwrap_err()
        .to_string()
        .contains("Layout resource limit exceeded"));
    let (bold, bytes) = bounded_layout(&nested(r"\pmb{", "}", 15));
    assert!(bytes < BUDGET, "nested bold allocated {bytes} bytes");
    ALLOCATED.store(0, Relaxed);
    ARMED.store(true, Relaxed);
    assert_eq!(to_display_list(&bold).items.len(), 1 << 15);
    ARMED.store(false, Relaxed);
    for rule in [r"\hline ", r"\hdashline "] {
        let ruled_bold = |count| {
            format!(
                r"{}\begin{{array}}{{c}}{}x\end{{array}}{}",
                r"\pmb{".repeat(13),
                rule.repeat(count),
                "}".repeat(13)
            )
        };
        // The accepted boundary still renders every rule in every layer.
        let (ruled, _) = bounded_layout(&ruled_bold(3));
        ALLOCATED.store(0, Relaxed);
        ARMED.store(true, Relaxed);
        assert_eq!(to_display_list(&ruled).items.len(), 4 * (1 << 13));
        ARMED.store(false, Relaxed);
        for count in [4, 64, 300] {
            assert!(parse(&ruled_bold(count))
                .unwrap_err()
                .to_string()
                .contains("Layout resource limit exceeded"));
        }
    }
    for n in [33, 300] {
        assert!(parse(&nested(r"\pmb{", "}", n))
            .unwrap_err()
            .to_string()
            .contains("Recursion limit exceeded"));
    }
    let sparse = |n: usize| {
        format!(
            r"\begin{{matrix}}{}x{}\end{{matrix}}",
            "&".repeat(n - 1),
            r"\\x".repeat(n - 1)
        )
    };
    let (_, small) = bounded_layout(&sparse(250));
    let (large, large_bytes) = bounded_layout(&sparse(500));
    assert!(
        large_bytes < small * 3,
        "sparse growth: {small} -> {large_bytes}"
    );
    assert_eq!(to_display_list(&large).items.len(), 500);
    let (middle, _) = bounded_layout(&nested(r"\left(", r"\middle|x\right)", 32));
    assert!(to_display_list(&middle).items.len() >= 32);
    for n in [33, 300] {
        assert!(parse(&nested(r"\left(", r"\middle|x\right)", n))
            .unwrap_err()
            .to_string()
            .contains("Recursion limit exceeded"));
    }
    // Temporary ASTs created by font/HTML wrappers must also reuse safely.
    let wrapped = nested(
        r"\left(\htmlStyle{font-weight:bold;}{",
        r"}\middle|x\right)",
        10,
    );
    bounded_layout(&wrapped);
}

#[test]
fn resource_amplification_is_bounded_in_an_isolated_process() {
    const ENV: &str = "RATEX_RESOURCE_CHILD";
    if std::env::var_os(ENV).is_some() {
        if cfg!(debug_assertions) {
            run_cases();
        } else {
            std::thread::Builder::new()
                .stack_size(512 * 1024)
                .spawn(run_cases)
                .unwrap()
                .join()
                .unwrap();
        }
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "resource_amplification_is_bounded_in_an_isolated_process",
            "--nocapture",
        ])
        .env(ENV, "1")
        .status()
        .unwrap();
    assert!(
        status.success(),
        "resource regression child failed: {status}"
    );
}
