#![no_std]

pub mod types;

pub use types::{ShellCommand, ShellCommandParseError, parse_command};

#[cfg(test)]
mod tests;
