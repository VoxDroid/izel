use crate::Pass;

pub struct EscapePass;

impl<IR> Pass<IR> for EscapePass {
    fn name(&self) -> &str {
        "escape"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
