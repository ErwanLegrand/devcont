#![no_main]

use devcont::devcontainers::one_or_many::OneOrMany;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = OneOrMany::parse_str(s);
    }
});
