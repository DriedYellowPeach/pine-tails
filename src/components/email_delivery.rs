use base64::prelude::*;
use lettre::message::{header, Message, MultiPart, SinglePart};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretBox};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::OnceCell;

use crate::{configuration::GmailApiSettings, domain::users::UserEmail};

#[derive(thiserror::Error, Debug)]
pub enum EmailClientError {
    #[error(transparent)]
    CommunicationError(#[from] reqwest::Error),
    #[error("{0}")]
    DataFormatError(String),
    #[error(transparent)]
    EmailFormatError(#[from] lettre::error::Error),
}

#[derive(Default, Debug)]
pub struct EmailClient {
    access_token: OnceCell<SecretBox<String>>,
    base_url: String,
    client_id: String,
    client_secret: SecretBox<String>,
    http_client: Client,
    refresh_url: String,
    refresh_token: SecretBox<String>,
    sender: UserEmail,
}

impl From<&GmailApiSettings> for EmailClient {
    fn from(settings: &GmailApiSettings) -> Self {
        Self::new(
            settings
                .access_token
                .as_ref()
                .map(|x| x.expose_secret().to_string()),
            settings.email_api.clone(),
            settings.token_api.clone(),
            settings.refresh_token.expose_secret().to_string(),
            settings.client_id.clone(),
            settings.client_secret.expose_secret().to_string(),
            settings.sender().unwrap(),
        )
    }
}

impl EmailClient {
    pub fn new(
        access_token: Option<String>,
        base_url: String,
        refresh_url: String,
        refresh_token: String,
        client_id: String,
        client_secret: String,
        sender: UserEmail,
    ) -> Self {
        Self {
            access_token: OnceCell::new_with(access_token.map(|x| SecretBox::new(Box::new(x)))),
            base_url,
            client_id,
            client_secret: SecretBox::new(Box::new(client_secret)),
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
            refresh_url,
            refresh_token: SecretBox::new(Box::new(refresh_token)),
            sender,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_refresh_url(mut self, refresh_url: String) -> Self {
        self.refresh_url = refresh_url;
        self
    }

    pub fn with_sender(mut self, sender: UserEmail) -> Self {
        self.sender = sender;
        self
    }

    pub fn with_access_token(mut self, access_token: String) -> Self {
        self.access_token = OnceCell::new_with(Some(SecretBox::new(Box::new(access_token))));
        self
    }

    // TODO: Also handle expiry, set a timer to refresh, my paln is make client a single service,
    // there is an event loop in client, and there are two kinds of event: SEND_EMAIL and
    // UPDATE_TOKEN. there should be channels for sending event
    #[tracing::instrument(name = "Fetching Acess Token")]
    pub async fn fetch_acess_token(&self) -> Result<SecretBox<String>, EmailClientError> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.expose_secret()),
            ("refresh_token", self.refresh_token.expose_secret()),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http_client
            .post(&self.refresh_url)
            .form(&params)
            .send()
            .await?
            .error_for_status()?;

        let response_body: HashMap<String, serde_json::Value> = response.json().await?;

        response_body
            .get("access_token")
            .map(|token| {
                tracing::info!("Fetched access token");
                SecretBox::new(Box::new(token.as_str().unwrap().to_string()))
            })
            .ok_or_else(|| {
                tracing::error!("Failed to find access_token field in the body of response");
                EmailClientError::DataFormatError("Failed to find access_token field".to_string())
            })
    }

    pub async fn send_email(
        &self,
        recipient: &UserEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), EmailClientError> {
        let message = Message::builder()
            .from(self.sender.as_ref().parse().unwrap())
            .to(recipient.as_ref().parse().unwrap())
            .subject(subject)
            .multipart(
                MultiPart::alternative() // This is composed of two parts.
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(String::from(text_content)), // Every message should have a plain text fallback.
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(String::from(html_content)),
                    ),
            )?;

        let raw_message = BASE64_STANDARD.encode(message.formatted());
        let email_body = json!({
            "raw": raw_message
        });

        let access_token = self
            .access_token
            .get_or_try_init(|| self.fetch_acess_token())
            .await?;

        let _response = self
            .http_client
            .post(&self.base_url)
            .bearer_auth(access_token.expose_secret()) // Use the OAuth2 access token
            .json(&email_body) // Send the JSON request body
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::configuration::get_configurations;
    use crate::domain::users::UserEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate, Times};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let body: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            match body {
                Ok(body) => body.get("raw").is_some(),
                Err(_) => false,
            }
        }
    }

    async fn mocking_refresh_ok<T: Into<Times>>(svr: &MockServer, hit: T) {
        let json_response = json!({
            "access_token": "new_refresh_token",
            "expiry": "2024-10-10T03:51:54.065331Z",
        });

        Mock::given(method("POST"))
            .and(path("/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json_response))
            .expect(hit)
            .mount(svr)
            .await;
    }

    async fn mocking_refresh_err<T: Into<Times>>(svr: &MockServer, hit: T) {
        Mock::given(method("POST"))
            .and(path("/refresh"))
            .respond_with(ResponseTemplate::new(500))
            .expect(hit)
            .mount(svr)
            .await;
    }

    async fn mocking_send_mail_ok<T: Into<Times>>(svr: &MockServer, hit: T) {
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(header_exists("authorization"))
            .and(header("Content-Type", "application/json"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(hit)
            .mount(svr)
            .await;
    }

    async fn mocking_send_mail_err<T: Into<Times>>(svr: &MockServer, hit: T) {
        Mock::given(method("POST"))
            .and(path("/send"))
            .and(header_exists("authorization"))
            .and(header("Content-Type", "application/json"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(hit)
            .mount(svr)
            .await;
    }

    fn fake_sender() -> UserEmail {
        UserEmail::try_from(SafeEmail().fake::<String>()).unwrap()
    }

    fn fake_receiver() -> UserEmail {
        UserEmail::try_from(SafeEmail().fake::<String>()).unwrap()
    }

    fn fake_subject() -> String {
        Sentence(1..2).fake()
    }

    fn fake_content() -> String {
        Paragraph(1..10).fake()
    }

    async fn do_send_email(client: &EmailClient) -> Result<(), EmailClientError> {
        client
            .send_email(
                &fake_receiver(),
                &fake_subject(),
                &fake_content(),
                &fake_content(),
            )
            .await
    }

    #[tokio::test]
    async fn send_email_should_fail_if_api_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        mocking_refresh_ok(&mock_server, 1).await;
        mocking_send_mail_err(&mock_server, 1).await;

        let email_client = EmailClient::default()
            .with_base_url(format!("{}/send", mock_server.uri()))
            .with_refresh_url(format!("{}/refresh", mock_server.uri()))
            .with_sender(fake_sender());

        // Act
        let response = do_send_email(&email_client).await;

        // Assert
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        mocking_refresh_ok(&mock_server, 1).await;
        mocking_send_mail_ok(&mock_server, 1).await;

        let email_client = EmailClient::default()
            .with_base_url(format!("{}/send", mock_server.uri()))
            .with_refresh_url(format!("{}/refresh", mock_server.uri()))
            .with_sender(fake_sender());

        // Act
        let response = do_send_email(&email_client).await;

        // Assert
        assert!(response.is_ok());
        assert!(email_client.access_token.get().is_some());
        assert_eq!(
            email_client.access_token.get().unwrap().expose_secret(),
            "new_refresh_token"
        )
    }

    #[tokio::test]
    async fn init_email_client_with_access_token_should_skip_refresh() {
        let mock_server = MockServer::start().await;
        mocking_refresh_ok(&mock_server, 0).await;
        mocking_send_mail_ok(&mock_server, 1).await;

        let email_client = EmailClient::default()
            .with_base_url(format!("{}/send", mock_server.uri()))
            .with_refresh_url(format!("{}/refresh", mock_server.uri()))
            .with_access_token("some_token".to_string())
            .with_sender(fake_sender());

        // Act
        let response = do_send_email(&email_client).await;

        // Assert
        assert!(response.is_ok());
        assert!(email_client.access_token.get().is_some());
        assert_eq!(
            email_client.access_token.get().unwrap().expose_secret(),
            "some_token"
        )
    }

    #[tokio::test]
    async fn refresh_access_token_fails_should_return_error() {
        let mock_server = MockServer::start().await;
        mocking_refresh_err(&mock_server, 1).await;
        mocking_send_mail_ok(&mock_server, 0).await;

        let email_client = EmailClient::default()
            .with_base_url(format!("{}/send", mock_server.uri()))
            .with_refresh_url(format!("{}/refresh", mock_server.uri()))
            .with_sender(fake_sender());

        // Act
        let response = do_send_email(&email_client).await;

        // Assert
        assert!(response.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn send_real_email_out_with_production_settings() {
        let config = get_configurations().unwrap();
        let email_client: EmailClient = (&config.gmail_service).into();
        let html_content = r#"
        <html>
        <body>
            <h1>Hello from Rust!</h1>
            <p>This is an <b>HTML</b> email sent via Gmail API.</p>
        </body>
        </html>
        "#;

        email_client
            .send_email(
                &"wang.runze4@northeastern.edu"
                    .to_string()
                    .try_into()
                    .unwrap(),
                &fake_subject(),
                html_content,
                &fake_content(),
            )
            .await
            .unwrap();
    }
}
