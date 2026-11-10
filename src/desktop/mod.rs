//! This computer's desktop as the daemon reaches it over the session bus.

pub mod prompt;

pub use prompt::{Prompt, PromptAction, PromptError, Prompts, DEFAULT_PROMPT_TIMEOUT};
