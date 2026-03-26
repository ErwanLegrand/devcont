//! Basic usage of the devcont library.
//!
//! This example demonstrates how to load user settings programmatically.
//!
//! ```bash
//! cargo run --example basic_usage
//! ```

fn main() -> anyhow::Result<()> {
    // Load user settings from ~/.config/devcont/config.toml
    let settings = devcont::settings::Settings::load()?;
    println!("Provider: {:?}", settings.provider);
    println!(
        "Dotfiles: {}",
        if settings.dotfiles.is_empty() {
            "(none)".to_string()
        } else {
            settings.dotfiles.join(", ")
        }
    );

    Ok(())
}
