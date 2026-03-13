pub mod detect;
pub mod faceapp;
pub mod localagent;
pub mod traits;
pub mod videoforge;

pub use detect::select_adapter;
pub use faceapp::FaceappAdapter;
pub use localagent::LocalAgentAdapter;
pub use traits::*;
pub use videoforge::VideoforgeAdapter;
