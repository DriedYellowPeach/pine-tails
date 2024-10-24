pub mod email_service_mocking;

use base64::prelude::*;
use mail_parser::MessageParser;
use once_cell::sync::Lazy;
use reqwest::Url;
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

use pine_tails::configuration::{get_configurations, DatabaseSettings};
use pine_tails::startup::engine::Engine as WebEngine;
use pine_tails::startup::prepare::{prepare_db_pool, prepare_email_client, Kits};
use pine_tails::telemetry::{get_subscriber, init_subscriber, LoggerFormat, LoggerOutbound};

static TRACING: Lazy<()> = Lazy::new(|| {
    let use_test_log = std::env::var("TEST_LOG").map_or(false, |x| {
        matches!(x.as_str(), "1" | "true" | "yes" | "TRUE")
    });

    let valid_levels = ["info", "error", "trace", "warn", "debug"];
    let level = std::env::var("LOG_LEVEL").ok();
    let log_level = level
        .as_deref()
        .filter(|lvl| valid_levels.contains(lvl))
        .unwrap_or("error");

    let format = LoggerFormat::Pretty;

    if use_test_log {
        let subscriber = get_subscriber(
            log_level.into(),
            format,
            LoggerOutbound::new(std::io::stderr),
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("debug".into(), format, LoggerOutbound::new(std::io::sink));
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub email_api: String,
    pub refresh_api: String,
    pub client: reqwest::Client,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn spawn_server() -> TestApp {
        Lazy::force(&TRACING);
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to create listener");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{}", port);

        let email_server = MockServer::start().await;

        let api_root = email_server.uri();
        let email_api = "email".to_string();
        let token_api = "refresh".to_string();

        let configuration = {
            let mut temp_config = get_configurations().expect("Failed to read configuration");
            temp_config.database.database_name = Uuid::new_v4().to_string();
            temp_config.gmail_service.email_api = format!("{}/{}", api_root, email_api);
            temp_config.gmail_service.token_api = format!("{}/{}", api_root, token_api);
            temp_config.application.base_url = address.clone();
            temp_config
        };

        let db_pool = Self::pool_to_uniq_database(&configuration.database).await;
        let test_app = TestApp {
            address,
            // this is the extra db pool we used to access directly
            // To check if data persists in db
            db_pool,
            email_server,
            email_api,
            refresh_api: token_api,
            client: reqwest::Client::new(),
        };

        let kits = Kits::new(
            listener,
            prepare_db_pool(&configuration),
            prepare_email_client(&configuration),
        );
        let engine = WebEngine::build(configuration, kits).unwrap();
        tokio::spawn(engine.spinup());

        test_app
    }

    async fn pool_to_uniq_database(config: &DatabaseSettings) -> PgPool {
        let mut connection =
            PgConnection::connect(config.connection_string_without_db().expose_secret())
                .await
                .expect("Failed to connect to Postgres");

        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
            .await
            .expect("Failed to create database.");

        // Migrate Database
        let connection_pool = PgPool::connect(config.connection_string().expose_secret())
            .await
            .expect("Failed to connect ot Postgres");

        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate database");

        connection_pool
    }

    pub async fn post_subscription<T: Serialize + ?Sized>(&self, form: &T) -> reqwest::Response {
        let api_addr = format!("{}/subscriptions", self.address);
        self.client
            .post(&api_addr)
            .form(form)
            .send()
            .await
            .expect("Failed to send request")
    }

    pub async fn post_newsletters<T: Serialize + ?Sized>(&self, body: &T) -> reqwest::Response {
        let api_addr = format!("{}/newsletters", self.address);
        self.client
            .post(&api_addr)
            .json(body)
            .send()
            .await
            .expect("Failed to send request")
    }

    pub async fn request_resend_email(&self, email: &str) -> reqwest::Response {
        let api_addr = format!("{}/subscriptions/resend_confirmation", self.address);
        let form = [("email", email)];

        self.client
            .post(&api_addr)
            .form(&form)
            .send()
            .await
            .expect("Failed to send request")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        // ---> Email Restore
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let raw_mail = BASE64_STANDARD
            .decode(body["raw"].as_str().unwrap())
            .unwrap();
        let message = MessageParser::default().parse(&raw_mail).unwrap();
        // <---

        let get_link = |s: &str| {
            let links = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect::<Vec<_>>();
            assert_eq!(links.len(), 1);
            links[0].as_str().to_owned()
        };
        let html_link = get_link(message.body_html(0).unwrap().as_ref());
        let html_confirmation_link = Url::parse(&html_link).unwrap();
        assert_eq!(html_confirmation_link.host_str().unwrap(), "127.0.0.1");

        let plain_text_link = get_link(message.body_text(0).unwrap().as_ref());
        let plain_confirmation_link = Url::parse(&plain_text_link).unwrap();
        assert_eq!(plain_confirmation_link.host_str().unwrap(), "127.0.0.1");

        ConfirmationLinks {
            html: html_confirmation_link,
            plain_text: plain_confirmation_link,
        }
    }
}
