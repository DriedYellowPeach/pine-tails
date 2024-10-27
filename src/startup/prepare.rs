use secrecy::ExposeSecret;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};

use std::net::TcpListener;

use crate::{components::email_delivery::EmailClient, configuration::Settings};

pub struct Kits {
    pub listener: TcpListener,
    pub db_pool: PgPool,
    pub email_client: EmailClient,
}

impl Kits {
    pub fn new(listener: TcpListener, db_pool: PgPool, email_client: EmailClient) -> Self {
        Self {
            listener,
            db_pool,
            email_client,
        }
    }

    pub fn prepare(config: &Settings) -> Result<Self, std::io::Error> {
        Ok(Self {
            listener: prepare_listener(config)?,
            db_pool: prepare_db_pool(config),
            email_client: prepare_email_client(config),
        })
    }
}

pub fn prepare_listener(config: &Settings) -> Result<TcpListener, std::io::Error> {
    let address = format!("{}:{}", config.application.host, config.application.port);
    TcpListener::bind(address)
}

pub fn prepare_db_pool(config: &Settings) -> PgPool {
    let options = PgConnectOptions::new()
        .host(&config.database.host)
        .port(config.database.port)
        .username(&config.database.username)
        .password(config.database.password.expose_secret())
        .database(&config.database.database_name);

    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(options)
}

pub fn prepare_email_client(config: &Settings) -> EmailClient {
    EmailClient::from(&config.gmail_service)
}
