use crate::Pass;

pub struct TcoPass;

impl<IR> Pass<IR> for TcoPass {
    fn name(&self) -> &str {
        "tco"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
