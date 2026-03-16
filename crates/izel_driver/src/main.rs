use clap::Parser;
use izel_session::{Session, SessionOptions};
use anyhow::Result;

fn main() -> Result<()> {
    let options = SessionOptions::parse();
    let _session = Session::new(options);

    println!("⬡ Izel Compiler (izelc) — Foundation Scaffolding Complete.");
    println!("Creator: @VoxDroid <izeno.contact@gmail.com>");
    println!("Repository: https://github.com/VoxDroid/izel");
    
    // Future pipeline stages will be invoked here.
    
    Ok(())
}
