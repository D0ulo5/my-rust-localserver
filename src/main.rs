mod config;
mod server;

use config::Config;
use server::Server;

fn main() {
    println!("Starting LocalServer...");

    // Load configuration
    let config = match Config::from_file("config.yaml") {
        Ok(cfg) => {
            println!("Configuration loaded successfully!");
            println!("Host: {}", cfg.host);
            println!("Ports: {:?}", cfg.ports);
            println!("Routes: {} routes configured", cfg.routes.len());
            cfg
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Create and run the server
    let mut server = match Server::new(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };

    println!("Server is running. Press Ctrl+C to stop.");

    // Run event loop (this runs forever)
    if let Err(e) = server.run() {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}