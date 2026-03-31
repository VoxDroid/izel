use izel_opt::{default_mir_pass_manager, IdentityPass, Pass, PassManager};
use std::sync::atomic::{AtomicBool, Ordering};

struct IncrementOnce(AtomicBool);

impl Pass<Vec<i32>> for IncrementOnce {
    fn name(&self) -> &str {
        "increment_once"
    }

    fn run(&self, ir: &mut Vec<i32>) -> bool {
        if self.0.swap(true, Ordering::SeqCst) {
            return false;
        }
        ir.push(1);
        true
    }
}

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

#[test]
fn pass_manager_default_and_identity_pass_are_callable() {
    let pm = PassManager::<i32>::default();
    assert!(pm.passes.is_empty());

    let pass = IdentityPass;
    assert_eq!(<IdentityPass as Pass<i32>>::name(&pass), "identity");
    let mut ir = 7;
    assert!(!<IdentityPass as Pass<i32>>::run(&pass, &mut ir));
    assert_eq!(ir, 7);
}

#[test]
fn pass_manager_repeats_until_no_changes() {
    let mut pm = PassManager::<Vec<i32>>::new();
    pm.add(IncrementOnce(AtomicBool::new(false)));
    pm.add(IdentityPass);

    let mut ir = Vec::new();
    pm.run(&mut ir);
    assert_eq!(ir, vec![1]);
}
