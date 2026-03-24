mod const_fold;
mod dce;
mod escape;
mod gvn;
mod inline;
mod iter_fuse;
mod licm;
mod sroa;
mod tco;

pub use const_fold::ConstFoldPass;
pub use dce::DcePass;
pub use escape::EscapePass;
pub use gvn::GvnPass;
pub use inline::InlinePass;
pub use iter_fuse::IterFusePass;
pub use licm::LicmPass;
pub use sroa::SroaPass;
pub use tco::TcoPass;
