use actix_web::HttpResponse;

#[tracing::instrument(name = "Health Check")]
pub async fn health_check() -> HttpResponse {
    tracing::info!("Health check request confirmed");
    HttpResponse::Ok().finish()
}
