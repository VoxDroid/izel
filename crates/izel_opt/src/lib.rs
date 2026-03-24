pub mod pass;
pub mod passes;

pub use pass::{IdentityPass, Pass, PassManager};

use passes::{
    ConstFoldPass, DcePass, EscapePass, GvnPass, InlinePass, IterFusePass, LicmPass, SroaPass,
    TcoPass,
};

/// Build the default MIR optimizer pipeline.
///
/// The pass list mirrors the roadmap/checklist target for the Phase 6 optimizer.
pub fn default_mir_pass_manager<IR>() -> PassManager<IR> {
    let mut pm = PassManager::new();
    pm.add(ConstFoldPass);
    pm.add(DcePass);
    pm.add(InlinePass);
    pm.add(LicmPass);
    pm.add(TcoPass);
    pm.add(IterFusePass);
    pm.add(EscapePass);
    pm.add(SroaPass);
    pm.add(GvnPass);
    pm
}
