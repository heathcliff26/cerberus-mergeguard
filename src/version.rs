use std::process;

#[cfg(test)]
mod test;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: Option<&str> = option_env!("CI_COMMIT_SHA");

fn version_information() -> String {
    let mut info = format!("{NAME}:\n    Version: v{VERSION}\n");
    let mut commit = COMMIT.unwrap_or("unknown");
    if commit.len() > 7 {
        commit = &commit[..7];
    }
    info.push_str(&format!("    Commit: {commit}"));

    info
}

pub fn print_version_and_exit() {
    println!("{}", version_information());
    process::exit(0);
}
