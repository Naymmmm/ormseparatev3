extern crate winres;
use std::path::Path;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        
        // Only set manifest file if it exists
        if Path::new("app.manifest").exists() {
            res.set_manifest_file("app.manifest");
        }
        
        // Only set icon if it exists
        if Path::new("app.ico").exists() {
            res.set_icon("app.ico");
        }
        
        // The metadata from Cargo.toml's [package.metadata.winres] section
        // will be automatically included
        
        res.compile().expect("Failed to compile Windows resources");
    }
}