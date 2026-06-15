mod config;

use config::Config;

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
    
    // TODO: Start server with event loop
    println!("Server starting on {}:{:?}", config.host, config.ports);
}