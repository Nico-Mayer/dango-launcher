use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use cargo_packager_updater::semver::Version;
use cargo_packager_updater::url::Url;
use cargo_packager_updater::{Config, Update, Updater, UpdaterBuilder};

const ENDPOINT: &str =
    "https://github.com/Nico-Mayer/dango-launcher/releases/latest/download/latest.json";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDZBRTJCQTVCOEQ0OUZDODkKUldTSi9FbU5XN3JpYXNrWTh3ZjNNMW1UT0puNDU2Q0F0TG5TTzZtMTNNQThWeDJaT2tmVEtKQW8K";

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

/// Where updates come from. The GitHub releases in production, a fake in
/// tests.
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

/// Starts Dango again from the application the update was installed into.
pub type Restart = Box<dyn Fn(&Path) + Send + Sync>;

/// The release source in a release build, nothing in a development build.
pub fn production(restart: Restart) -> std::sync::Arc<dyn UpdateSource> {
    if cfg!(debug_assertions) {
        std::sync::Arc::new(DisabledSource)
    } else {
        std::sync::Arc::new(PackagerSource::new(restart))
    }
}

/// Never finds an update. A development build has no release to update to.
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

/// The signed releases on GitHub, through `cargo-packager-updater`.
pub struct PackagerSource {
    found: Mutex<Option<Update>>,
    restart: Restart,
}

impl PackagerSource {
    pub fn new(restart: Restart) -> Self {
        Self {
            found: Mutex::new(None),
            restart,
        }
    }

    fn found(&self) -> Option<Update> {
        self.found.lock().unwrap().clone()
    }
}

fn updater() -> cargo_packager_updater::Result<Updater> {
    let config = Config {
        endpoints: vec![Url::parse(ENDPOINT)?],
        pubkey: PUBLIC_KEY.into(),
        windows: None,
    };
    UpdaterBuilder::new(Version::parse(env!("CARGO_PKG_VERSION"))?, config).build()
}

#[async_trait]
impl UpdateSource for PackagerSource {
    async fn check(&self) -> Result<Option<String>, UpdateError> {
        let found = tokio::task::spawn_blocking(|| updater()?.check())
            .await
            .map_err(|error| UpdateError::Check(error.to_string()))?
            .map_err(|error| UpdateError::Check(error.to_string()))?;
        let version = found.as_ref().map(|update| update.version.clone());
        *self.found.lock().unwrap() = found;
        Ok(version)
    }

    async fn download(&self, progress: Progress<'_>) -> Result<Vec<u8>, UpdateError> {
        let update = self
            .found()
            .ok_or_else(|| UpdateError::Download("no update was found to download".into()))?;
        let (sender, mut chunks) = tokio::sync::mpsc::unbounded_channel();
        let download = tokio::task::spawn_blocking(move || {
            update.download_extended(
                |chunk, total| {
                    let _ = sender.send((chunk as u64, total));
                },
                || {},
            )
        });
        while let Some((chunk, total)) = chunks.recv().await {
            progress(chunk, total);
        }
        download
            .await
            .map_err(|error| UpdateError::Download(error.to_string()))?
            .map_err(download_error)
    }

    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError> {
        let update = self
            .found()
            .ok_or_else(|| UpdateError::Install("no update was found to install".into()))?;
        // On Windows this exits, and the installer starts Dango again, so the
        // restart only runs on macOS.
        update
            .install(bytes.to_vec())
            .map_err(|error| UpdateError::Install(error.to_string()))?;
        (self.restart)(&update.extract_path);
        Ok(())
    }
}

fn download_error(error: cargo_packager_updater::Error) -> UpdateError {
    use cargo_packager_updater::Error;

    let detail = error.to_string();
    match error {
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) => {
            UpdateError::Signature(detail)
        }
        _ => UpdateError::Download(detail),
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    #[test]
    fn the_public_key_decodes() {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(PUBLIC_KEY)
            .unwrap();
        minisign_verify::PublicKey::decode(std::str::from_utf8(&decoded).unwrap()).unwrap();
    }

    #[test]
    fn the_updater_builds_with_the_running_version() {
        updater().unwrap();
    }

    #[test]
    fn signature_failures_are_signature_errors() {
        use cargo_packager_updater::Error;

        for error in [
            Error::Minisign(minisign_verify::Error::InvalidSignature),
            Error::Base64(base64::DecodeError::InvalidLength(3)),
            Error::SignatureUtf8("not a signature".into()),
        ] {
            assert!(matches!(download_error(error), UpdateError::Signature(_)));
        }
    }

    #[test]
    fn other_download_failures_are_download_errors() {
        let error = cargo_packager_updater::Error::Network("status 404".into());
        assert!(matches!(download_error(error), UpdateError::Download(_)));
    }

    #[tokio::test]
    async fn a_development_build_has_the_disabled_source() {
        let source = production(Box::new(|_| panic!("a development build restarted")));
        assert_eq!(source.check().await, Ok(None));
        assert_eq!(
            source.download(&mut |_, _| {}).await,
            Err(UpdateError::Download("updates are disabled".into()))
        );
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
