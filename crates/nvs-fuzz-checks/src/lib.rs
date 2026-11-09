//! Each fuzz target's check, as one function that takes the target's input
//! bytes and panics where a property fails.
//!
//! The targets under `fuzz/fuzz_targets/` are one line each, a call to the
//! function here of the same name. This crate's own test replays every saved
//! input under `fuzz/seeds/<target>/` through the same function, so the inputs
//! a search found are checked by plain `cargo test` on every platform, and a
//! search runs only when somebody starts one (`bun nv fuzz <target>`).
//!
//! Each check holds safety properties: no panic on any input, and the
//! invariants a reader of outside bytes promises about what it gives. One
//! target stays out: `markdown_comrak` compares two engines' renderings, and
//! is a target to run by hand.
//!
//! The crate holds no `unsafe`. A check that calls a `Core` member owns each
//! value the call returns through `nvs_stdlib::fuzz::Owned`, which releases it.

mod ast;
mod cookie;
mod decompress;
mod image_path;
mod json;
mod lex;
mod markdown;
mod markdown_from_html;
mod multipart;
mod parse;
mod prefix;
mod regex;
mod reply_head;
mod request_head;
mod serialize;
mod uri;
mod xml;

#[cfg(test)]
mod tests;

pub use ast::ast;
pub use cookie::cookie;
pub use decompress::decompress;
pub use image_path::image_path;
pub use json::json;
pub use lex::lex;
pub use markdown::markdown;
pub use markdown_from_html::markdown_from_html;
pub use multipart::multipart;
pub use parse::parse;
pub use prefix::prefix;
pub use regex::regex;
pub use reply_head::reply_head;
pub use request_head::request_head;
pub use serialize::serialize;
pub use uri::uri;
pub use xml::xml;
