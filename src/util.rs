use std::sync::{Arc, OnceLock};

/// A thread-safe, lazily initialized value.
/// The value is initialized on the first call to `init`.
/// Subsequent calls to `init` will return an error.
/// The value can be accessed via `get` after initialization.
#[derive(Debug, Clone)]
pub struct LazyInit<T> {
    inner: Arc<OnceLock<T>>,
}

impl<T> LazyInit<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OnceLock::new()),
        }
    }

    /// Initialize the value.
    /// Returns an error if the value has already been initialized.
    pub fn init(&self, value: T) -> Result<(), T> {
        self.inner.set(value)
    }

    /// Get a reference to the initialized value.
    /// Panics if the value has not been initialized.
    pub fn get(&self) -> &T {
        self.inner
            .get()
            .expect("LazyInit used before initialization")
    }
}
