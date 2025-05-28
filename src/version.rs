use std::process;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_version_and_exit() {
    println!("{NAME}:");
    println!("    Version: v{VERSION}");
    // TODO: Implement commit version
    process::exit(0);
}
