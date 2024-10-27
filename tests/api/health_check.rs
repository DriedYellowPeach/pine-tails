use crate::utils::TestApp;

#[tokio::test]
async fn health_check() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/health_check", app.address);

    let response = app
        .client
        .get(api_addr)
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn healthcheck_should_return_200_even_if_url_with_extra_trailing_slash() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/health_check/", app.address);

    let response = app
        .client
        .get(api_addr)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(Some(0), response.content_length());
}
