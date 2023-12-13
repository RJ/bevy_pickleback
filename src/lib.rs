mod client_plugin;
mod server_plugin;

pub mod prelude {
    pub use super::client_plugin::*;
    pub use super::server_plugin::*;
    pub use pickleback::prelude::*;
}
