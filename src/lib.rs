//! rust-dev-template - Complete Rust development environment
//!
//! This template provides a comprehensive Rust development setup with:
//! - Development container for consistent environments
//! - Pre-commit hooks for code quality
//! - `XTask` commands for development workflows
//! - All tools integrated and ready to use
//!
//! Copyright (c) 2025 Erwan Patrick Legrand
//!
//! Licensed under the MIT OR Apache-2.0 License.
//! You may obtain a copy of the License at
//!
//! - MIT: <https://opensource.org/licenses/MIT>
//! - Apache-2.0: <https://www.apache.org/licenses/LICENSE-2.0>
//!
//! Unless required by applicable law or agreed to in writing, software
//! distributed under the License is distributed on an "AS IS" BASIS,
//! WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//! See the License for the specific language governing permissions and
//! limitations under the License.

//! # Examples
//!
//! ```
//! use rust_dev_template::prelude::*;
//!
//! // Your code here
//! # Ok::<(), rust_dev_template::Error>(())
//! ```

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod prelude;

// Re-export commonly used items
pub use error::{Error, Result};
pub use prelude::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_initialization() {
        let result = Ok::<(), crate::Error>(());
        assert!(result.is_ok());
    }
}
