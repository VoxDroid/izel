use crate::Pass;

pub struct IterFusePass;

impl<IR> Pass<IR> for IterFusePass {
    fn name(&self) -> &str {
        "iter_fuse"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
