//! Inline run formatting, display projection, and alignment model primitives.

pub mod alignment;
pub mod display;
mod normalize;
pub mod paragraphs;
pub mod runs;

pub use alignment::*;
pub use display::*;
pub use normalize::*;
pub use paragraphs::*;
pub use runs::*;
