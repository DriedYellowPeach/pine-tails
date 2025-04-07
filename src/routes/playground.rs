use actix_web::{http, web, HttpResponse, ResponseError};
use base64::{engine::general_purpose, Engine as _};
use nn_rs::nn::NN;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum PlaygroundError {
    #[error("{0}")]
    RecognizeError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PlaygroundError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::RecognizeError(_) => http::StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum GrayScaleImg {
    Base64(String),
    Vector(Vec<u8>),
}

impl GrayScaleImg {
    pub fn to_vec(self) -> Vec<u8> {
        match self {
            GrayScaleImg::Base64(base64_str) => {
                let base64_data = base64_str.trim_start_matches("data:image/png;base64,");
                let decoded_bytes = general_purpose::STANDARD.decode(base64_data).unwrap();
                let img = image::load_from_memory(&decoded_bytes).expect("Failed to decode image");
                let gray_values = img.to_luma8().pixels().map(|p| p[0]).collect::<Vec<u8>>();

                gray_values
            }
            GrayScaleImg::Vector(vec) => vec,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecognizeInfo {
    pub digit: usize,
    pub confidence: f32,
}

#[tracing::instrument(name = "Recognize Handwriting Digit", skip(img, nn))]
pub async fn recognize_digit(
    nn: web::Data<NN>,
    img: web::Json<GrayScaleImg>,
) -> Result<HttpResponse, PlaygroundError> {
    let img = img.into_inner().to_vec();
    let input_size = nn.input_size();
    // TODO: match the input size of nn
    if img.len() != input_size {
        return Err(PlaygroundError::RecognizeError(format!(
            "Image must have {input_size} pixels"
        )));
    }
    let ret = nn_rs::recognize_digit(&nn, &img);

    tracing::info!(target: "Recognizing digit", "digit: {}, with confidence: {}", ret.0, ret.1);

    // Ok(HttpResponse::Ok().json(serde_json::json!({
    //     "platform": "macos",
    //     "accelerate": "Blas Apple Accelerate",
    //     "digit": ret.0.to_string(),
    //     "confidence": ret.1.to_string(),
    // })))
    Ok(HttpResponse::Ok().json(RecognizeInfo {
        digit: ret.0,
        confidence: ret.1,
    }))
}
