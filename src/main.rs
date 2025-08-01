use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::io::{self, Write, BufRead, Read};

use anyhow::{Result, Context, anyhow};
use clap::{Parser, Subcommand};
use image::{GenericImageView, ImageBuffer, Rgba};
use regex::Regex;
use serde::{Deserialize, Serialize};
use rayon::prelude::*;
use walkdir::WalkDir;

// Configuration structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChannelConfig {
    name: String,
    channel: usize, // 0 = R, 1 = G, 2 = B
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Profile {
    name: String,
    file_regex: String,
    output_format: String,
    channels: Vec<ChannelConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    default_profile: String,
    profiles: HashMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        
        // Default ORM profile
        let default_profile = Profile {
            name: "orm".to_string(),
            file_regex: "/orm/i".to_string(),  // New format: /pattern/args
            output_format: "png".to_string(),
            channels: vec![
                ChannelConfig { name: "Occlusion".to_string(), channel: 0 },
                ChannelConfig { name: "Roughness".to_string(), channel: 1 },
                ChannelConfig { name: "Metallic".to_string(), channel: 2 },
            ],
        };
        
        profiles.insert("orm".to_string(), default_profile);
        
        Config {
            default_profile: "orm".to_string(),
            profiles,
        }
    }
}

impl Config {
    fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            
            Ok(config)
        } else {
            // Create default config if it doesn't exist
            println!("Config file not found. Creating default config at: {}", path.display());
            
            let config = Config::default();
            let toml = toml::to_string_pretty(&config)
                .context("Failed to serialize default config")?;
            
            fs::write(path, toml)
                .with_context(|| format!("Failed to write default config to: {}", path.display()))?;
            
            println!("Default config created successfully with 'orm' profile.");
            
            Ok(config)
        }
    }
    
    fn get_profile(&self, profile_name: Option<&str>) -> Result<Profile> {
        let profile_name = profile_name.unwrap_or(&self.default_profile);
        
        self.profiles.get(profile_name)
            .cloned()
            .ok_or_else(|| anyhow!("Profile '{}' not found in config", profile_name))
    }
}

// CLI Arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Input files or directories to process
    inputs: Vec<PathBuf>,
    
    /// Profile to use for processing
    #[arg(short, long)]
    profile: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available profiles
    ListProfiles,
}

// Display help information when no arguments are provided
fn display_help() {
    println!("ORM Separator V3");
    println!("=================");
    println!("Made with ❤️ by Darwin");
    println!();
    println!("Usage:");
    println!("  ormseparatev3 [OPTIONS] [INPUTS]...");
    println!("  ormseparatev3 list-profiles");
    println!();
    println!("Examples:");
    println!("  ormseparatev3 image.png                    # Process a single image");
    println!("  ormseparatev3 --profile custom folder/     # Process a folder with custom profile");
    println!("  ormseparatev3 list-profiles                # List available profiles");
    println!();
    println!("You can also drag and drop files or folders onto the executable.");
    println!();
    println!("Options:");
    println!("  -p, --profile <PROFILE>    Profile to use for processing (default: orm)");
    println!("  -h, --help                 Show this help message");
    println!("  -V, --version              Show version information");
}

// Wait for a keypress from the user
fn wait_for_keypress() -> Result<()> {
    println!("\nPress any key to continue...");
    io::stdout().flush().context("Failed to flush stdout")?;
    
    // Read a single byte from stdin
    let mut buffer = [0; 1];
    io::stdin().read(&mut buffer).context("Failed to read user input")?;
    
    Ok(())
}

// Parse regex in /pattern/args format
fn parse_regex_format(regex_str: &str) -> Result<(&str, &str)> {
    // Check if the string follows the /pattern/args format
    if regex_str.starts_with('/') {
        // Find the position of the second slash
        if let Some(second_slash_pos) = regex_str[1..].find('/') {
            // +1 because we're searching in the substring starting at index 1
            let second_slash_pos = second_slash_pos + 1;
            
            // Extract pattern and flags
            let pattern = &regex_str[1..second_slash_pos];
            let flags = &regex_str[second_slash_pos + 1..];
            
            return Ok((pattern, flags));
        }
    }
    
    // If the string doesn't follow the /pattern/args format, treat it as a regular regex pattern
    Ok((regex_str, ""))
}

// Prompt user to select a profile when multiple profiles exist
fn prompt_profile_selection(config: &Config) -> Result<String> {
    println!("\nMultiple profiles available. Please select a profile to use:");
    
    // Display available profiles
    let mut profile_names: Vec<String> = config.profiles.keys().cloned().collect();
    profile_names.sort(); // Sort alphabetically for consistent display
    
    for (i, name) in profile_names.iter().enumerate() {
        let is_default = if name == &config.default_profile { " (default)" } else { "" };
        println!("  {}. {}{}", i + 1, name, is_default);
    }
    
    // Prompt for selection
    print!("\nEnter profile number or name (default is {}): ", config.default_profile);
    io::stdout().flush().context("Failed to flush stdout")?;
    
    // Read user input
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).context("Failed to read user input")?;
    let input = input.trim();
    
    // If empty input, use default
    if input.is_empty() {
        return Ok(config.default_profile.clone());
    }
    
    // Try to parse as number
    if let Ok(num) = input.parse::<usize>() {
        if num > 0 && num <= profile_names.len() {
            return Ok(profile_names[num - 1].clone());
        } else {
            println!("Invalid profile number. Using default profile: {}", config.default_profile);
            return Ok(config.default_profile.clone());
        }
    }
    
    // Try as profile name
    if config.profiles.contains_key(input) {
        return Ok(input.to_string());
    }
    
    // If not found, use default
    println!("Profile '{}' not found. Using default profile: {}", input, config.default_profile);
    Ok(config.default_profile.clone())
}

fn main() -> Result<()> {
    // Parse CLI arguments first
    let cli = Cli::parse();
    
    // Get the directory where the executable is located
    let exe_path = std::env::current_exe()
        .with_context(|| "Failed to get executable path")?;
    let exe_dir = exe_path.parent()
        .unwrap_or_else(|| Path::new("."));
    
    // Store config file in the same directory as the binary
    let config_path = exe_dir.join("config.toml");
    
    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }
    
    // Load or create config - this will create the config file if it doesn't exist
    let config = Config::load(&config_path)?;
    
    // Handle subcommands
    if let Some(Commands::ListProfiles) = cli.command {
        println!("Available profiles:");
        for (name, profile) in &config.profiles {
            println!("  {} - Regex: '{}', Format: '{}'", name, profile.file_regex, profile.output_format);
            println!("    Channels:");
            for channel in &profile.channels {
                println!("      {} (Channel: {})", channel.name, channel.channel);
            }
        }
        println!("\nDefault profile: {}", config.default_profile);
        return Ok(());
    }
    
    // Display help if no inputs were provided
    if cli.inputs.is_empty() {
        display_help();
        // Wait for keypress before exiting when showing help due to no arguments
        wait_for_keypress()?;
        return Ok(());
    }
    
    // Get the profile to use
    let profile_name = if cli.profile.is_none() && config.profiles.len() > 1 {
        // If no profile specified and multiple profiles exist, prompt for selection
        let selected_profile = prompt_profile_selection(&config)?;
        Some(selected_profile)
    } else {
        cli.profile
    };
    
    let profile = config.get_profile(profile_name.as_deref())?;
    
    println!("Using profile: {}", profile.name);
    
    // Process inputs
    for input in cli.inputs {
        process_input(&input, &profile)?;
    }
    
    Ok(())
}

// Process a single input (file or directory)
fn process_input(input: &Path, profile: &Profile) -> Result<()> {
    if input.is_dir() {
        process_directory(input, profile)
    } else {
        process_file(input, profile)
    }
}

// Process a directory recursively
fn process_directory(dir: &Path, profile: &Profile) -> Result<()> {
    println!("Processing directory: {}", dir.display());
    
    // Parse regex in /pattern/args format
    let (pattern, flags) = parse_regex_format(&profile.file_regex)?;
    
    // Create regex with appropriate options
    let regex = if flags.contains('i') {
        Regex::new(&format!("(?i){}", pattern))
    } else {
        Regex::new(pattern)
    }.with_context(|| format!("Invalid regex pattern: {}", profile.file_regex))?;
    
    let files: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file() && 
            regex.is_match(&entry.path().to_string_lossy())
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();
    
    println!("Found {} matching files", files.len());
    
    // Process files in parallel
    files.par_iter()
        .try_for_each(|file| process_file(file, profile))?;
    
    Ok(())
}

// Process a single file
fn process_file(file: &Path, profile: &Profile) -> Result<()> {
    println!("Processing file: {}", file.display());
    
    // Load the image
    let img = image::open(file)
        .with_context(|| format!("Failed to open image: {}", file.display()))?;
    
    // Get the file stem and parent directory
    let file_stem = file.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid file name: {}", file.display()))?;
    
    let parent = file.parent().unwrap_or(Path::new("."));
    
    // Process each channel
    for channel_config in &profile.channels {
        let channel_idx = channel_config.channel;
        if channel_idx > 2 {
            return Err(anyhow!("Invalid channel index: {}", channel_idx));
        }
        
        // Create a new image with just this channel
        let (width, height) = img.dimensions();
        let mut channel_img = ImageBuffer::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                let channel_value = pixel[channel_idx];
                
                // Set all channels to the same value to create a grayscale image
                channel_img.put_pixel(x, y, Rgba([channel_value, channel_value, channel_value, 255]));
            }
        }
        
        // Create output filename
        let output_filename = format!("{}_{}.{}", file_stem, channel_config.name, profile.output_format);
        let output_path = parent.join(output_filename);
        
        // Save the channel image
        channel_img.save(&output_path)
            .with_context(|| format!("Failed to save channel image: {}", output_path.display()))?;
        
        println!("  Saved channel {} to: {}", channel_config.name, output_path.display());
    }
    
    Ok(())
}
