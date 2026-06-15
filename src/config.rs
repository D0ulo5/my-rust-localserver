use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub host: String,
    pub ports: Vec<u16>,
    pub client_max_body_size: usize,
    #[serde(default)]
    pub error_pages: HashMap<u16, String>,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    pub path: String,
    #[serde(default)]
    pub methods: Vec<String>,  // Make methods optional with default
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub directory_listing: bool,
    #[serde(default)]
    pub default_file: Option<String>,
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub permanent: Option<bool>,
    #[serde(default)]
    pub cgi_extension: Option<String>,
    #[serde(default)]
    pub upload_enabled: bool,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: Config = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;
        
        config.validate()?;
        Ok(config)
    }
    
    fn validate(&self) -> Result<(), String> {
        // Check for duplicate ports
        let mut ports_seen = std::collections::HashSet::new();
        for port in &self.ports {
            if !ports_seen.insert(port) {
                return Err(format!("Duplicate port: {}", port));
            }
        }
        
        // Validate client body size
        if self.client_max_body_size == 0 {
            return Err("client_max_body_size must be greater than 0".to_string());
        }
        
        // Validate each route
        for route in &self.routes {
            // Check path starts with /
            if !route.path.starts_with('/') {
                return Err(format!("Route path must start with '/': {}", route.path));
            }
            
            // Handle redirect routes
            if route.redirect.is_some() {
                // Redirect routes don't need root or methods
                continue;
            }
            
            // Non-redirect routes need a root
            if route.root.is_none() {
                return Err(format!("Route '{}' must have 'root' (or be a redirect)", route.path));
            }
            
            // Validate methods for non-redirect routes
            if route.methods.is_empty() {
                return Err(format!("Route '{}' must have at least one method", route.path));
            }
            
            // If root is set, check directory exists
            if let Some(root) = &route.root {
                let path = Path::new(root);
                if !path.exists() {
                    return Err(format!("Root directory '{}' for route '{}' does not exist", root, route.path));
                }
                if !path.is_dir() {
                    return Err(format!("Root path '{}' for route '{}' is not a directory", root, route.path));
                }
            }
            
            // Validate methods are uppercase and valid
            for method in &route.methods {
                let upper = method.to_uppercase();
                if upper != "GET" && upper != "POST" && upper != "DELETE" {
                    return Err(format!("Invalid method '{}' in route '{}'", method, route.path));
                }
            }
            
            // If CGI extension is set, ensure it starts with dot
            if let Some(ext) = &route.cgi_extension {
                if !ext.starts_with('.') {
                    return Err(format!("CGI extension '{}' in route '{}' must start with '.'", ext, route.path));
                }
            }
        }
        
        Ok(())
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation_duplicate_ports() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080, 8080]
client_max_body_size: 1024
routes:
  - path: "/"
    methods: ["GET"]
    root: "./www"
"#;
        let config: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_invalid_method() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/test"
    methods: ["INVALID"]
    root: "./www"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_missing_root() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/test"
    methods: ["GET"]
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_redirect_route() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/old"
    redirect: "http://localhost:8080/new"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation_valid() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/"
    methods: ["GET"]
    root: "./www"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation_cgi_extension() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/cgi"
    methods: ["GET"]
    root: "./cgi-bin"
    cgi_extension: ".py"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation_invalid_cgi_extension() {
        let yaml = r#"
host: "127.0.0.1"
ports: [8080]
client_max_body_size: 1024
routes:
  - path: "/cgi"
    methods: ["GET"]
    root: "./cgi-bin"
    cgi_extension: "py"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }
}