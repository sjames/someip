pub struct CallProperties {
    pub timeout: std::time::Duration,
}

impl Default for CallProperties {
    fn default() -> Self {
        CallProperties {
            timeout: std::time::Duration::from_secs(1),
        }
    }
}

impl CallProperties {
    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            timeout: timeout,
            ..Default::default()
        }
    }
}
