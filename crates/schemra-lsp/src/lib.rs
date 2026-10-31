//! Language Server Protocol support for Schemra source projects.

mod backend;
mod positions;
mod server;

#[cfg(test)]
mod tests;

/// Runs the Schemra language server over standard input and output.
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run_stdio()
}
