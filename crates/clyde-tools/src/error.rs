use std::fmt;

// Perhaps actually used in the future.
enum ToolErrorInner {
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// A tool error.
pub struct ToolError {
    inner: ToolErrorInner,
}

impl ToolError {
    /// Create a tool error from the inner enumeration.
    #[inline]
    const fn new(inner: ToolErrorInner) -> Self {
        Self { inner }
    }

    /// Create a "other" tool error from any error.
    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let error = error.into();

        Self::new(ToolErrorInner::Other(error))
    }
}

impl fmt::Debug for ToolError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ToolErrorInner::Other(error) = &self.inner;

        fmt::Debug::fmt(error, fmt)
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ToolErrorInner::Other(error) = &self.inner;

        fmt::Display::fmt(error, fmt)
    }
}

impl std::error::Error for ToolError {}
