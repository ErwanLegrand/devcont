#![no_main]

use devcont::devcontainers::config::Config;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = Config::parse_str(s);
    }
});
