use crate::Pass;

pub struct GvnPass;

impl<IR> Pass<IR> for GvnPass {
    fn name(&self) -> &str {
        "gvn"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
