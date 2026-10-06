use css::{Site, sparse_site::SourceFile};
use css::{content_hash::FingerprintBits, content_name::ContentName};
use std::sync::Arc;

#[test]
fn variable_length_is_bounded_when_unnumbered_source_is_100_kb() {
    // Given: a large received source and the largest supported position/role.
    let source = "// filler\n".repeat(10_240);
    let site = Site {
        file: SourceFile::Unnumbered(Arc::from(source)),
        at: usize::MAX,
        role: usize::MAX,
    };
    // When: the production naming API emits its variable.
    let name = site.variable_name("test-");
    // Then: source size cannot inflate the file part beyond 18 bytes.
    let digits = if usize::BITS == 64 { 13 } else { 7 };
    assert!(
        name.len() <= 4 + 5 + 18 + 2 + 2 * digits,
        "length={}",
        name.len()
    );
    assert_eq!(name.len(), 4 + 5 + 18 + 2 + 2 * digits);
}

#[test]
fn lossless_wins_when_source_payload_ties_the_fingerprint_length() {
    // Given: normalized source payloads at either side of the 16-byte boundary.
    let sources = ["a".repeat(15), "a".repeat(16), "a".repeat(17)];
    // When: the shared min-length algorithm names each source.
    let names = sources.map(|source| ContentName::source(&source).name(""));
    // Then: ties stay lossless and only the longer source uses a fingerprint.
    assert_eq!(names[0], format!("UL{}", "a".repeat(15)));
    assert_eq!(names[1], format!("UL{}", "a".repeat(16)));
    assert!(names[2].starts_with("UH"));
    assert_eq!(names[2].len(), 18);
}

#[test]
fn source_fingerprint_is_fixed_when_source_has_multiple_sha_blocks() {
    // Given: the NIST two-block input, independent of Rust/compiler versions.
    let source = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
    // When: the source is named without a filename or diagnostic witness.
    let name = ContentName::source(source).name("");
    // Then: the first 80 SHA-256 bits match the independent fixed byte golden.
    assert_eq!(name, "UHc8yjo0690l8os8yl");
}

#[test]
fn narrow_width_names_collide_when_distinct_sources_have_the_same_digest_prefix() {
    // Given: three distinct sources competing for two one-bit fingerprints.
    let bits = FingerprintBits::new(1).unwrap_or_else(|| panic!("valid width"));
    let sites = ["// source zero", "// source one", "// source two"].map(|source| Site {
        file: SourceFile::Unnumbered(Arc::from(source)),
        at: 10,
        role: 2,
    });
    // When: the production function receives the explicit narrow width.
    let names = sites.map(|site| site.variable_name_with_bits("", bits));
    // Then: the collision path is reachable without globals or a test-only hasher.
    assert!(names[0] == names[1] || names[0] == names[2] || names[1] == names[2]);
    assert!(names.iter().all(|name| name.starts_with("---SUH")));
}
