use crate::config::Config;
use crate::identifiers::acoustid::AcoustIdIdentifier;
use crate::identifiers::acrcloud::AcrCloudIdentifier;
use crate::identifiers::backend::Identifier;
use crate::identifiers::shazam::ShazamIdentifier;
use std::sync::Arc;

pub struct IdentifierRegistry {
    pub backends: Vec<Arc<dyn Identifier>>,
}

impl IdentifierRegistry {
    pub fn from_config(config: &Config) -> Self {
        let mut backends: Vec<Arc<dyn Identifier>> = Vec::new();

        if let Some(shazam_cfg) = config.shazam.clone() {
            backends.push(Arc::new(ShazamIdentifier::new(shazam_cfg)));
        }

        if let Some(acoustid_cfg) = config.acoustid.clone() {
            backends.push(Arc::new(AcoustIdIdentifier::new(acoustid_cfg)));
        }

        if let Some(acr_cfg) = config.acrcloud.clone() {
            backends.push(Arc::new(AcrCloudIdentifier::new(acr_cfg)));
        }

        if backends.is_empty() {
            // Fallback: Shazam sem config funciona na API web pública.
            backends.push(Arc::new(ShazamIdentifier::new(
                crate::config::ShazamConfig {},
            )));
        }

        Self { backends }
    }

    pub fn enabled_backends(&self) -> Vec<&'static str> {
        self.backends
            .iter()
            .filter(|b| b.enabled())
            .map(|b| b.name())
            .collect()
    }
}
