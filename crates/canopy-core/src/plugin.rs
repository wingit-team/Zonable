//! `Plugin` trait — modular engine feature registration.
//!
//! Each engine subsystem (renderer, audio, sim, etc.) and each game feature
//! is a `Plugin`. Plugins call into `CanopyApp`'s builder methods during
//! the `build` phase (before the event loop starts).
//!
//! # Ordering
//!
//! Plugins are built in registration order. Plugins that depend on others
//! should be registered after their dependencies. A future phase will add
//! explicit dependency declarations via `Plugin::dependencies() -> Vec<TypeId>`.

use crate::app::CanopyApp;

/// The plugin trait. Implement this to add reusable engine features.
pub trait Plugin: Send + Sync + 'static {
    /// Called once during `CanopyApp::run()`, before the event loop starts.
    /// Register systems, resources, and event handlers here.
    fn build(&self, app: &mut CanopyApp);

    /// Human-readable name for debug output.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// A `Plugin` implemented by a closure — for quick one-off registrations.
///
/// ```rust
/// app.add_plugin(fn_plugin(|app| {
///     app.add_system(SystemStage::Update, my_fn_system);
/// }));
/// ```
pub struct FnPlugin<F: Fn(&mut CanopyApp) + Send + Sync + 'static>(pub F);

impl<F: Fn(&mut CanopyApp) + Send + Sync + 'static> Plugin for FnPlugin<F> {
    fn build(&self, app: &mut CanopyApp) {
        (self.0)(app);
    }
}

pub fn fn_plugin<F: Fn(&mut CanopyApp) + Send + Sync + 'static>(f: F) -> FnPlugin<F> {
    FnPlugin(f)
}
