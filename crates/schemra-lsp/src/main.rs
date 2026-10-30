fn main() {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--help" | "-h") => {
            println!(
                "Usage: schemra-lsp\n\nStarts the Schemra language server over standard input and output."
            );
            return;
        }
        Some(argument) => {
            eprintln!("unknown schemra-lsp argument: {argument}");
            std::process::exit(2);
        }
    }

    if let Err(error) = schemra_lsp::run_stdio() {
        eprintln!("Schemra language server stopped: {error}");
        std::process::exit(1);
    }
}
