use crate::Pass;

pub struct ConstFoldPass;

impl<IR> Pass<IR> for ConstFoldPass {
    fn name(&self) -> &str {
        "const_fold"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
