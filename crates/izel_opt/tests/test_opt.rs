use izel_opt::default_mir_pass_manager;

#[test]
fn full_mir_optimizer_registers_expected_passes() {
    let pm = default_mir_pass_manager::<()>();
    let names: Vec<&str> = pm.passes.iter().map(|p| p.name()).collect();
    assert_eq!(
        names,
        vec![
            "const_fold",
            "dce",
            "inline",
            "licm",
            "tco",
            "iter_fuse",
            "escape",
            "sroa",
            "gvn",
        ]
    );
}

#[test]
fn full_mir_optimizer_runs_on_ir() {
    let pm = default_mir_pass_manager::<Vec<i32>>();
    let mut ir = vec![1, 2, 3];
    pm.run(&mut ir);
    assert_eq!(ir, vec![1, 2, 3]);
}
