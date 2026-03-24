use crate::Pass;

pub struct LicmPass;

impl<IR> Pass<IR> for LicmPass {
    fn name(&self) -> &str {
        "licm"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
