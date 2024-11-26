use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretBox};

use crate::domain::users::UserEmail;
use crate::telemetry::LoggerFormat;

#[derive(serde::Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: AppSettings,
    pub gmail_service: GmailApiSettings,
    pub blob_storage: BlobStorageSettings,
}

#[derive(serde::Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretBox<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct AppSettings {
    pub port: u16,
    pub host: String,
    pub logger_format: LoggerFormat,
    pub base_url: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct BlobStorageSettings {
    pub base_dir: PathBuf,
    #[serde(default)]
    pub ephemeral: bool,
}

#[derive(serde::Deserialize, Debug)]
pub struct GmailApiSettings {
    pub sender_email: String,
    pub token_api: String,
    pub email_api: String,
    pub client_id: String,
    pub client_secret: SecretBox<String>,
    pub refresh_token: SecretBox<String>,
    pub access_token: Option<SecretBox<String>>,
}

impl GmailApiSettings {
    pub fn sender(&self) -> Result<UserEmail, String> {
        self.sender_email.clone().try_into()
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> SecretBox<String> {
        SecretBox::new(Box::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )))
    }

    pub fn connection_string_without_db(&self) -> SecretBox<String> {
        SecretBox::new(Box::new(format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )))
    }
}

pub fn get_configurations() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configurations");

    let builder = config::Config::builder()
        // Add in `./Settings.toml`
        .add_source(
            config::File::with_name(
                configuration_directory
                    .join("base")
                    .to_str()
                    .expect("Path contains invalid unicode."),
            )
            .required(false),
        );

    let builder = builder.add_source(
        config::File::with_name(
            configuration_directory
                .join("secret")
                .to_str()
                .expect("Path contains invalid unicode."),
        )
        .required(false),
    );

    // If it is in a CI, also load CI configurations
    let is_ci = std::env::var("RUN_CI").map_or(false, |s| {
        matches!(s.as_str(), "1" | "true" | "yes" | "TRUE" | "YES")
    });

    let builder = if is_ci {
        builder.add_source(
            config::File::with_name(
                configuration_directory
                    .join("ci-base")
                    .to_str()
                    .expect("Path contains invalid unicode."),
            )
            .required(false),
        )
    } else {
        builder
    };

    let environment: Environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let builder = builder.add_source(
        config::File::with_name(
            configuration_directory
                .join(environment.as_str())
                .to_str()
                .expect("Path contains invalid unicode."),
        )
        .required(false),
    );

    let settings = builder.build().unwrap();

    settings.try_deserialize()
}

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_configuration_successfully_with_secrete_yml() {
        let config = get_configurations();
        assert!(config.is_ok());
    }

    #[test]
    fn test_load_ci_configuration_successfully() {
        std::env::set_var("RUN_CI", "true");
        let config = get_configurations().unwrap();
        assert_eq!(config.database.port, 5432);
    }

    #[test]
    fn test_path_join() {
        use std::path::{Path, PathBuf};
        assert_eq!(
            Path::new("/noexist")
                .join("passwd")
                .to_str()
                .expect("path is not valid unicode"),
            PathBuf::from("/noexist/passwd")
                .to_str()
                .expect("path is not valid unicode")
        );
    }
}
