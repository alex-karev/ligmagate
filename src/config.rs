use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

/// Config file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    providers: HashMap<String, Provider>,
    #[serde(default)]
    models: HashMap<String, Model>,
    #[serde(default)]
    variants: HashMap<String, Variants>,
    env_file: Option<PathBuf>,
    port: Option<u16>,
    host: Option<String>,
    #[serde(default)]
    no_reload: bool,
    api_key: Option<String>,
    #[serde(skip)]
    env: HashMap<String, String>,
}

/// Model provider api
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Provider {
    #[serde(rename = "type", default)]
    kind: ProviderKind,
    api_base: Option<String>,
    api_key: Option<String>,
}

/// Models with routing to providers
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Model {
    #[serde(default)]
    routing: Vec<String>,
    #[serde(default)]
    providers: HashMap<String, ProviderModel>,
}

/// Model variants with custom parameters and system prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Variants {
    #[serde(rename = "type", default)]
    kind: VariantKind,
    model: String,
    system_prompt: Option<PathBuf>,
    #[serde(default)]
    system_prompt_mode: PromptMode,
}

/// Per-provider settings for models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderModel {
    model_name: Option<String>,
    #[serde(default)]
    extra_body: Vec<ExtraBody>,
}

/// Extra request body
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtraBody {
    pointer: String,
    value: toml::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ProviderKind {
    #[default]
    OpenaiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum VariantKind {
    #[default]
    ChatCompletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PromptMode {
    Replace,
    Combine,
    #[default]
    Fallback,
}

impl Config {
    /// Load config from path
    pub async fn load(path: &PathBuf) -> anyhow::Result<Self> {
        // Expand shell path
        let path = PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).as_ref());

        // Scan input path
        let entries: Vec<_> = WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Generate empty config
        if entries.is_empty() {
            info!(
                "No toml files found at '{:?}'. Generating default config.",
                &path
            );
            fs::create_dir_all(&path).await?;
            let default_str = toml::to_string_pretty(&Self::default())?;
            let default_path = path.join("config.toml");
            fs::write(&default_path, &default_str).await?;
            return Ok(Self::default());
        }

        // Read files
        let mut contents = Vec::new();
        for path in entries {
            match fs::read_to_string(path).await {
                Ok(s) => contents.push(s),
                Err(e) => error!("Failed to read config: {}", e),
            }
        }

        // Merge configs
        let merged = contents.join("\n");
        let mut config: Config = toml::from_str(&merged)
            .inspect_err(|e| error!("Error parsing config: {}.\nEmpty config loaded", e))
            .unwrap_or(Self::default());

        // Load env
        if config.env_file.is_some() {
            config.load_env();
        }

        // Validate
        if !config.validate(&path).await {
            warn!("Config validation did not pass successfully. Check logs for more info.")
        }

        Ok(config)
    }
    /// Validation logic
    pub async fn validate(&self, path: &PathBuf) -> bool {
        let mut valid = true;
        // Check models
        for (model_name, model) in self.models.iter() {
            for provider in model.routing.iter() {
                if !self.providers.contains_key(provider.as_str()) {
                    error!("Model {} uses unknown provider {}", model_name, provider);
                    valid = false;
                }
            }
            for (provider_name, provider) in model.providers.iter() {
                if !model.routing.contains(provider_name) {
                    debug!(
                        "Extra configuration found for provider {}, but it is not enabled",
                        provider_name
                    );
                }
                if provider.model_name.is_none() {
                    debug!(
                        "Provider-specific model name for model {} by provider {} is not specified. Using {} as a model_name",
                        model_name, provider_name, model_name
                    );
                }
            }
        }

        // Check providers
        for (provider_name, provider) in self.providers.iter() {
            match provider.kind {
                ProviderKind::OpenaiCompatible => {
                    if provider.api_base.is_none() {
                        error!(
                            "Provider {} of type openai_compatible requires api_base",
                            provider_name
                        );
                        valid = false;
                    }
                    if provider.api_key.is_none() {
                        error!(
                            "Provider {} of type openai_compatible requires api_key",
                            provider_name
                        );
                        valid = false;
                    }
                }
            }
        }

        // Check variants
        for (variant_name, variant) in self.variants.iter() {
            if !self.models.contains_key(&variant.model) {
                error!(
                    "Variant {} uses unspecified model {}",
                    variant_name, &variant.model
                );
                valid = false;
            }

            if let Some(system_prompt) = &variant.system_prompt {
                let system_prompt_path = path.join(system_prompt);
                if fs::try_exists(&system_prompt_path).await.is_err() {
                    error!(
                        "System prompt template for variant {} is specified, but was not found in {:?}",
                        variant_name, &system_prompt_path
                    );
                    valid = false;
                }
            }
        }

        valid
    }

    /// Returns application port
    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(6969)
    }

    /// Returns application host
    pub fn get_host(&self) -> String {
        self.host.clone().unwrap_or(String::from("0.0.0.0"))
    }

    /// Returns application api key
    pub fn get_api_key(&self) -> Option<String> {
        self.api_key.as_ref().and_then(|key| {
            if let Some(var_name) = key.strip_prefix("env:") {
                env::var(var_name).ok()
            } else {
                Some(key.clone())
            }
        })
    }

    /// Checks if hot-reload is enabled
    pub fn is_reload_enabled(&self) -> bool {
        !self.no_reload
    }

    // Get env file path
    pub fn get_env_file(&self) -> Option<PathBuf> {
        self.env_file.clone()
    }

    // Load env
    pub fn load_env(&mut self) {
        // Skip if nothing to load
        let Some(path) = &self.env_file else { return };

        // Expand env path
        let path = PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).as_ref());

        // Read env file
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read env file '{}': {}", path.display(), e);
                return;
            }
        };

        // Read env file line by line
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            self.env
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    // Get env variable
    pub fn get_env(&self, key: &str) -> Option<String> {
        if let Some(value) = self.env.get(key) {
            Some(value.clone())
        } else {
            env::var(key).ok()
        }
    }
}
