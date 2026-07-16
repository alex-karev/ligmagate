use anyhow::{Result, anyhow};
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
    temperature: Option<f32>,
    #[serde(default)]
    extra_headers: HashMap<String, String>,
    #[serde(default)]
    extra_body: Vec<ExtraBody>,
}

/// Model variants with custom parameters and system prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Variants {
    model: String,
    system_prompt: Option<PathBuf>,
    #[serde(default)]
    system_prompt_mode: PromptMode,
    #[serde(default)]
    extra_headers: HashMap<String, String>,
    #[serde(default)]
    extra_body: Vec<ExtraBody>,
    temperature: Option<f32>,
}

/// Per-provider settings for models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderModel {
    model_name: Option<String>,
    #[serde(default)]
    extra_headers: HashMap<String, String>,
    #[serde(default)]
    extra_body: Vec<ExtraBody>,
}

/// Extra request body
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtraBody {
    pointer: String,
    value: toml::Value,
}

/// Extra request body with json values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraBodyJson {
    pub pointer: String,
    pub value: serde_json::Value,
}

/// Type of the API used by provider
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    OpenaiCompatible,
}

/// Prompt conflict handling mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    Replace,
    Combine,
    #[default]
    Fallback,
}

/// API-agnostic request forwarder data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestData {
    pub provider_kind: ProviderKind,
    pub api_base: String,
    pub api_key: String,
    pub model_name: String,
    pub extra_body: Vec<ExtraBodyJson>,
    pub extra_headers: HashMap<String, String>,
    pub system_prompt: Option<PathBuf>,
    pub system_prompt_mode: PromptMode,
    pub temperature: Option<f32>,
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
        self.port.unwrap_or(3000)
    }

    /// Returns application host
    pub fn get_host(&self) -> String {
        self.host.clone().unwrap_or(String::from("localhost"))
    }

    /// Returns application api key
    pub fn get_api_key(&self) -> Option<String> {
        self.api_key.as_ref().and_then(|key| {
            if let Some(var_name) = key.strip_prefix("env::") {
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

    // Get forwarder data
    pub fn get_data(&self, model_name: &str) -> Result<RequestData> {
        // Check variant
        let variant = self.variants.get(model_name);

        // Get model
        let model_name = if variant.is_some() {
            &variant.unwrap().model
        } else {
            model_name
        };
        let model = self
            .models
            .get(model_name)
            .ok_or(anyhow!("Model {} not defined", model_name))?;

        // Get provider
        // TODO: Add proper routing
        let provider_name = model
            .routing
            .first()
            .ok_or(anyhow!("Model {} has no providers", model_name))?;
        let provider = self
            .providers
            .get(provider_name)
            .ok_or(anyhow!("Provider {} not found", provider_name))?;
        let provider_kind = provider.kind.clone();

        // Get base url and api key
        let api_base = match provider_kind {
            ProviderKind::OpenaiCompatible => provider
                .api_base
                .clone()
                .ok_or(anyhow!("No url specified for provider {}", provider_name))?,
        };
        let api_key = provider.api_key.clone().unwrap_or("".to_string());
        let api_key = if api_key.starts_with("env::") {
            self.get_env(&api_key.strip_prefix("env::").unwrap())
                .unwrap_or("".to_string())
        } else {
            api_key
        };

        // Add extra body and headers
        let mut extra_body = Vec::new();
        let mut extra_headers = HashMap::new();
        extra_body.extend(model.extra_body.clone());
        extra_headers.extend(model.extra_headers.clone());
        if let Some(model_provider) = model.providers.get(provider_name) {
            extra_body.extend(model_provider.extra_body.clone());
            extra_headers.extend(model_provider.extra_headers.clone());
        }
        if let Some(v) = variant {
            extra_body.extend(v.extra_body.clone());
            extra_headers.extend(v.extra_headers.clone());
        }

        // Convert extra body to json
        let extra_body = extra_body.iter().map(|x| ExtraBodyJson {
            pointer: x.pointer.clone(),
            value: toml_to_json(&x.value),
        }).collect();

        // Add system prompt
        let (system_prompt, system_prompt_mode) = if let Some(v) = variant {
            (v.system_prompt.clone(), v.system_prompt_mode.clone())
        } else {
            (None, PromptMode::Fallback)
        };

        // Add other parameters
        let temperature = variant
            .map_or(model.temperature, |v| v.temperature.or(model.temperature))
            .or(model.temperature);

        // Check Provider-specific model name
        let model_name = if let Some(model_provider) = model.providers.get(provider_name) {
            model_provider.model_name.as_deref().unwrap_or(model_name)
        } else {
            model_name
        }
        .to_string();

        // Return forwarder data
        Ok(RequestData {
            provider_kind,
            api_base,
            api_key,
            extra_body,
            extra_headers,
            temperature,
            model_name,
            system_prompt,
            system_prompt_mode,
        })
    }
}

// Convert toml values to json
fn toml_to_json(val: &toml::Value) -> serde_json::Value {
    match val {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => {
            serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(0.into()))
        }
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(a) => serde_json::Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => serde_json::Value::Object(
            t.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}
