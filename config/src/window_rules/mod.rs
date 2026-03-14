use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::{ConfigGet, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

pub const ID: &str = "com.system76.CosmicSettings.WindowRules";

/// Gets a cosmic-config [Config] context.
pub fn context() -> Result<cosmic_config::Config, cosmic_config::Error> {
    Config::context()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DefaultApplicationException {
    pub appid: String,
    pub titles: Vec<String>,
}

impl DefaultApplicationException {
    pub fn expand(self) -> Vec<PreciseApplicationException> {
        self.titles
            .into_iter()
            .map(|title| PreciseApplicationException {
                appid: self.appid.clone(),
                title,
                enabled: true,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PreciseApplicationException {
    pub appid: String,
    pub title: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct WindowRuleException {
    pub appid: String,
    pub title: String,
    pub enabled: bool,
    pub workspace_id: usize,
    // TODO(karlskewes): what else? sticky, floating, etc..
}

// cosmic-config configuration state for `com.system76.CosmicSettings.WindowRules`
#[derive(Clone, Debug, Default, PartialEq, CosmicConfigEntry)]
#[version = 1]
pub struct Config {
    pub tiling_exception_defaults: Vec<DefaultApplicationException>,
    pub tiling_exception_custom: Vec<PreciseApplicationException>,
    // TODO(karlskewes): how does this get parsed from file?
    pub window_rule_custom: Vec<WindowRuleException>,
}

impl Config {
    pub fn context() -> Result<cosmic_config::Config, cosmic_config::Error> {
        cosmic_config::Config::new(ID, Self::VERSION)
    }
}

// This is the final struct that will be used by external modules
#[derive(Clone, Debug, PartialEq)]
pub struct ApplicationException {
    pub appid: String,
    pub title: String,
}

/// Get the current tiling exception configuration
///
/// Merges user-defined custom rules to the system default config
pub fn tiling_exceptions(context: &cosmic_config::Config) -> Vec<ApplicationException> {
    // Load shortcuts defined by the system.
    let defaults = context
        .get::<Vec<DefaultApplicationException>>("tiling_exception_defaults")
        .unwrap_or_else(|why| {
            tracing::error!("tiling exceptions defaults config error: {why:?}");
            Vec::new()
        });

    let mut expanded = defaults
        .into_iter()
        .flat_map(|exception| exception.expand())
        .collect::<Vec<PreciseApplicationException>>();

    // Load custom shortcuts defined by the user.
    let custom = context
        .get::<Vec<PreciseApplicationException>>("tiling_exception_custom")
        .unwrap_or_else(|why| {
            if why.is_err() {
                if let cosmic_config::Error::GetKey(_, err) = &why {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        tracing::error!("tiling exceptions custom config error: {why}");
                        return Vec::new();
                    }
                }
            }
            tracing::debug!("tiling exceptions custom config not present: {why}");
            Vec::new()
        });

    for exception in custom {
        if let Some(existing) = expanded
            .iter_mut()
            .find(|existing| existing.appid == exception.appid && existing.title == exception.title)
        {
            existing.enabled = exception.enabled;
        } else {
            expanded.push(PreciseApplicationException {
                appid: exception.appid,
                title: exception.title,
                enabled: exception.enabled,
            });
        }
    }

    expanded
        .iter()
        .filter_map(|exception| {
            if exception.enabled {
                Some(ApplicationException {
                    appid: exception.appid.clone(),
                    title: exception.title.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Get the current window rule exception configuration
pub fn window_rule_exceptions(context: &cosmic_config::Config) -> Vec<WindowRuleException> {
    // Load custom shortcuts defined by the user.
    let custom = context
        .get::<Vec<WindowRuleException>>("window_rule_custom")
        .unwrap_or_else(|why| {
            if why.is_err()
                && let cosmic_config::Error::GetKey(_, err) = &why
                && err.kind() != std::io::ErrorKind::NotFound
            {
                tracing::error!("window rule custom config error: {why}");
                return Vec::new();
            }
            tracing::debug!("window rule custom config not present: {why}");
            Vec::new()
        });

    custom
        .iter()
        .filter_map(|exception| {
            if exception.enabled {
                Some(WindowRuleException {
                    appid: exception.appid.clone(),
                    title: exception.title.clone(),
                    enabled: true, // TODO(karlskewes), this isn't needed here
                    // tiling_exceptions go through a precise (with enabled) -> default (without
                    // enabled). We could do the same but why if we're not merging. TBD.
                    workspace_id: exception.workspace_id,
                })
            } else {
                None
            }
        })
        .collect()
}
