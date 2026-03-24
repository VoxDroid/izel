use crate::Pass;

pub struct SroaPass;

impl<IR> Pass<IR> for SroaPass {
    fn name(&self) -> &str {
        "sroa"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
