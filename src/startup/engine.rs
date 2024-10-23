use actix_web::dev::Server;
use actix_web::middleware::{self, TrailingSlash};
use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::configuration::Settings;
use crate::routes::health_check;

use super::prepare::Kits;

pub struct Engine {
    web_server: Server,
}

pub struct WebBaseUrl(pub String);

impl Engine {
    pub fn build(config: Settings, kits: Kits) -> Result<Self, std::io::Error> {
        let db_pool = web::Data::new(kits.db_pool);
        let email_client = web::Data::new(kits.email_client);
        let base_url = web::Data::new(WebBaseUrl(config.application.base_url));

        let server = HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
                .route("/health_check", web::get().to(health_check))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
                .app_data(base_url.clone())
        })
        .listen(kits.listener)?
        .run();

        Ok(Self { web_server: server })
    }

    pub async fn spinup(self) -> Result<(), std::io::Error> {
        self.web_server.await
    }
}
