use actix_cors::Cors;
use actix_multipart::form::MultipartFormConfig;
use actix_web::dev::Server;
use actix_web::middleware::{self, TrailingSlash};
use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::configuration::Settings;
use crate::routes::*;

use super::prepare::Kits;

pub struct Engine {
    web_server: Server,
}

pub struct WebBaseUrl(pub String);

impl Engine {
    pub fn build(config: Settings, kits: Kits) -> Result<Self, std::io::Error> {
        let db_pool = web::Data::new(kits.db_pool);
        let email_client = web::Data::new(kits.email_client);
        let blob_storage = web::Data::new(kits.blob_storage);
        let base_url = web::Data::new(WebBaseUrl(config.application.base_url));

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::PayloadConfig::new(1024 * 1024 * 1024))
                .wrap(TracingLogger::default())
                .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
                .wrap(
                    Cors::default()
                        .allowed_origin("http://localhost:3000") // Replace with your frontend origin
                        .allow_any_method()
                        .allow_any_header()
                        .max_age(3600),
                )
                .service(
                    web::scope("/api")
                        .service(
                            web::scope("/posts")
                                .route("", web::get().to(get_all_posts))
                                .route("", web::post().to(upload_post))
                                .route("/{id}", web::put().to(update_post))
                                .route("/{id}", web::delete().to(delete_post))
                                .route("/slug/{slug}", web::get().to(get_post_by_slug))
                                .route(
                                    "/slug/{slug}/{attachment}",
                                    web::get().to(get_post_attachment),
                                )
                                .route("/count", web::get().to(posts_count)),
                        )
                        .route("/health_check", web::get().to(health_check)),
                )
                .app_data(MultipartFormConfig::default().total_limit(100 * 1024 * 1024))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
                .app_data(blob_storage.clone())
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
