use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate, Times};

use super::TestApp;

impl TestApp {
    pub async fn mocking_refresh_ok<T: Into<Times>>(&self, hit: T) {
        let json_response = json!({
            "access_token": "new_refresh_token",
            "expiry": "2024-10-10T03:51:54.065331Z",
        });

        Mock::given(method("POST"))
            .and(path(&self.refresh_api))
            .respond_with(ResponseTemplate::new(200).set_body_json(json_response))
            .expect(hit)
            .mount(&self.email_server)
            .await;
    }

    #[allow(dead_code)]
    pub async fn mocking_refresh_err<T: Into<Times>>(&self, hit: T) {
        Mock::given(method("POST"))
            .and(path(&self.refresh_api))
            .respond_with(ResponseTemplate::new(500))
            .expect(hit)
            .mount(&self.email_server)
            .await;
    }

    pub async fn mocking_send_mail_ok<T: Into<Times>>(&self, hit: T) {
        Mock::given(method("POST"))
            .and(path(&self.email_api))
            .respond_with(ResponseTemplate::new(200))
            .expect(hit)
            .mount(&self.email_server)
            .await;
    }

    pub async fn mocking_send_mail_err<T: Into<Times>>(&self, hit: T) {
        Mock::given(method("POST"))
            .and(path(&self.email_api))
            .respond_with(ResponseTemplate::new(500))
            .expect(hit)
            .mount(&self.email_server)
            .await;
    }
}
