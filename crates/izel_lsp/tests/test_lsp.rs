fn assert_debug<T: std::fmt::Debug>() {}

#[test]
fn backend_type_exposes_debug_contract() {
    assert_debug::<izel_lsp::Backend>();
}

#[test]
fn server_entry_points_are_exported() {
    let _sync: fn() = izel_lsp::run_server_sync;
    let future = izel_lsp::run_server();
    drop(future);
}
