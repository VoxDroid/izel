/// A transformation pass on the intermediate representation.
pub trait Pass<IR> {
    fn name(&self) -> &str;
    fn run(&self, ir: &mut IR) -> bool; // returns true if any changes were made
}

/// Manages the execution of multiple optimization passes.
pub struct PassManager<IR> {
    pub passes: Vec<Box<dyn Pass<IR>>>,
}

impl<IR> Default for PassManager<IR> {
    fn default() -> Self {
        Self::new()
    }
}

impl<IR> PassManager<IR> {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add<P: 'static + Pass<IR>>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    pub fn run(&self, ir: &mut IR) {
        let mut changed = true;
        while changed {
            changed = false;
            for pass in &self.passes {
                if pass.run(ir) {
                    changed = true;
                }
            }
        }
    }
}

/// A simple identity pass that does nothing.
pub struct IdentityPass;

impl<IR> Pass<IR> for IdentityPass {
    fn name(&self) -> &str {
        "identity"
    }

    fn run(&self, _ir: &mut IR) -> bool {
        false
    }
}
