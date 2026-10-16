//! Phase 5: IR -> style declarations -> semantic HTML.
//!
//! The layout IR is backend-agnostic. `backend.rs` defines the [`StyleSink`] abstraction; the
//! renderer (`html.rs`) computes every element's styling as a backend-neutral [`Style`] carrying
//! both a Tailwind utility string and the equivalent plain-CSS declarations, then the active sink
//! decides what to emit. `tailwind.rs` and `css.rs` hold the per-construct formatters.

pub mod backend;
pub mod css;
pub mod html;
pub mod tailwind;
