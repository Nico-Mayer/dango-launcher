use async_trait::async_trait;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("Couldn't check for updates. Check your connection and try again.")]
    Check(String),
    #[error("Couldn't download the update. Check your connection and try again.")]
    Download(String),
    #[error("Couldn't verify the update, so it wasn't installed. Try again later.")]
    Signature(String),
    #[error("Couldn't install the update. Try again later.")]
    Install(String),
}

impl UpdateError {
    /// The underlying cause, for the log rather than the screen.
    pub fn detail(&self) -> &str {
        match self {
            Self::Check(detail)
            | Self::Download(detail)
            | Self::Signature(detail)
            | Self::Install(detail) => detail,
        }
    }
}

/// Called with each chunk's size and the total size, when the server sent one.
pub type Progress<'a> = &'a mut (dyn FnMut(u64, Option<u64>) + Send);

/// Where updates come from. Nothing yet in production, a fake in tests.
#[async_trait]
pub trait UpdateSource: Send + Sync {
    /// The newer published version, if there is one.
    async fn check(&self) -> Result<Option<String>, UpdateError>;
    /// Downloads the update the last check found and verifies its signature.
    async fn download(&self, progress: Progress<'_>) -> Result<Vec<u8>, UpdateError>;
    /// Installs a verified download and restarts into it. Returns only on
    /// failure.
    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError>;
}

/// Stands in until Dango has an updater again: never finds an update.
pub struct DisabledSource;

#[async_trait]
impl UpdateSource for DisabledSource {
    async fn check(&self) -> Result<Option<String>, UpdateError> {
        Ok(None)
    }

    async fn download(&self, _progress: Progress<'_>) -> Result<Vec<u8>, UpdateError> {
        Err(UpdateError::Download("updates are disabled".into()))
    }

    fn install(&self, _bytes: &[u8]) -> Result<(), UpdateError> {
        Err(UpdateError::Install("updates are disabled".into()))
    }
}

#[cfg(test)]
pub mod testing {
    use std::sync::Mutex;

    use super::*;

    /// Answers from a script: what a check finds, and how a download and an
    /// install end. Records every request, so a test can tell none was made.
    #[derive(Default)]
    pub struct FakeSource {
        pub found: Mutex<Option<String>>,
        pub check_error: Mutex<Option<UpdateError>>,
        pub download_error: Option<UpdateError>,
        pub install_error: Option<UpdateError>,
        /// Chunk sizes the download reports, against `total`.
        pub chunks: Vec<u64>,
        pub total: Option<u64>,
        pub checks: Mutex<usize>,
        pub installed: Mutex<Vec<Vec<u8>>>,
    }

    impl FakeSource {
        pub fn finding(version: &str) -> Self {
            Self {
                found: Mutex::new(Some(version.into())),
                ..Default::default()
            }
        }

        pub fn checks(&self) -> usize {
            *self.checks.lock().unwrap()
        }
    }

    #[async_trait]
    impl UpdateSource for FakeSource {
        async fn check(&self) -> Result<Option<String>, UpdateError> {
            *self.checks.lock().unwrap() += 1;
            match self.check_error.lock().unwrap().clone() {
                Some(error) => Err(error),
                None => Ok(self.found.lock().unwrap().clone()),
            }
        }

        async fn download(&self, progress: Progress<'_>) -> Result<Vec<u8>, UpdateError> {
            for chunk in &self.chunks {
                progress(*chunk, self.total);
            }
            match &self.download_error {
                Some(error) => Err(error.clone()),
                None => Ok(b"update".to_vec()),
            }
        }

        fn install(&self, bytes: &[u8]) -> Result<(), UpdateError> {
            self.installed.lock().unwrap().push(bytes.to_vec());
            match &self.install_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }
    }
}
