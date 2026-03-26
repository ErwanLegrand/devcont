// Copyright (c) 2025 Erwan Patrick Legrand

//! Basic usage example for a library generated from rust-dev-template
//!
//! This example demonstrates the core functionality provided by the template.

use rust_dev_template::prelude::*;

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<()> {
    println!("Hello from rust-dev-template!");

    // Example of error handling
    match do_something() {
        Ok(result) => println!("Success: {result}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}

/// Example function that might return an error
#[allow(clippy::unnecessary_wraps)]
fn do_something() -> Result<String> {
    // Simulate some work that might fail
    Ok("Template working correctly!".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_something() {
        let result = do_something();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Template working correctly!");
    }
}
