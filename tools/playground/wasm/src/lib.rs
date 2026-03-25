use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn repl_eval(source: &str) -> String {
    if source.trim().is_empty() {
        return "error: source is empty".to_string();
    }

    // Phase 7 scaffold: frontend bridge returns parse-ready payload.
    format!("playground received {} bytes", source.len())
}
