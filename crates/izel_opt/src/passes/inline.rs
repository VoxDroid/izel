use crate::Pass;

pub struct InlinePass;

impl<IR> Pass<IR> for InlinePass {
    fn name(&self) -> &str {
        "inline"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
