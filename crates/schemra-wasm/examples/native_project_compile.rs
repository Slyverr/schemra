use std::{error::Error, io::Read};

use schemra_wasm::compile_project_schemra;

fn main() -> Result<(), Box<dyn Error>> {
    let mut project = String::new();
    std::io::stdin().read_to_string(&mut project)?;
    println!("{}", compile_project_schemra(&project));
    Ok(())
}
