use cerberus_mergeguard::App;
use clap::Parser;

#[tokio::main]
async fn main() {
    if let Err(e) = App::parse().run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
