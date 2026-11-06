use std::path::PathBuf;
use std::time::Duration;

use crate::extension::manifest::Manifest;
use crate::extension::Extension;

use super::manifest::JsManifest;

/// An extension loaded from a folder of the extensions directory.
pub struct JsExtension {
    manifest: Manifest,
    folder: PathBuf,
    refresh: Option<Duration>,
}

impl JsExtension {
    pub fn new(read: JsManifest, folder: PathBuf) -> Self {
        Self {
            manifest: read.manifest,
            folder,
            refresh: read.refresh,
        }
    }

    pub fn manifest_id(&self) -> &str {
        &self.manifest.id
    }

    pub fn folder(&self) -> &std::path::Path {
        &self.folder
    }

    pub fn refresh(&self) -> Option<Duration> {
        self.refresh
    }
}

impl Extension for JsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}
