use izel_std::add;
use std::path::PathBuf;

#[test]
fn add_returns_expected_sum() {
    assert_eq!(add(40, 2), 42);
    assert_eq!(add(0, 0), 0);
}

#[test]
fn critical_standard_library_files_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std");
    let required = [
        "iter.iz",
        "witness.iz",
        "thread.iz",
        "sync.iz",
        "atomic.iz",
        "test.iz",
        "bench.iz",
        "mock.iz",
    ];

    for name in required {
        let path = root.join(name);
        assert!(
            path.exists(),
            "expected std surface file to exist: {:?}",
            path
        );
    }
}
