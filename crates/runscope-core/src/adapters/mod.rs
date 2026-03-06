pub mod detect;
pub mod localagent;
pub mod traits;

pub use detect::select_adapter;
pub use localagent::LocalAgentAdapter;
pub use traits::*;
