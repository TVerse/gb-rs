mod execute;
mod fetch_decode;
pub mod instructions;
mod mod_old;

pub use execute::execute_instruction;
pub use fetch_decode::fetch_and_decode;
pub use fetch_decode::DecodeContext;
