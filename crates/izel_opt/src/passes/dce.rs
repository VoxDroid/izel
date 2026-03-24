use crate::Pass;

pub struct DcePass;

impl<IR> Pass<IR> for DcePass {
    fn name(&self) -> &str {
        "dce"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
