use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::models::Claims;

const JWT_SECRET: &[u8] = b"library-seat-reservation-secret-key-2024";

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let token = credentials.token();
    
    let decoding_key = DecodingKey::from_secret(JWT_SECRET);
    let validation = Validation::default();
    
    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            req.extensions_mut().insert(token_data.claims);
            Ok(req)
        }
        Err(e) => {
            eprintln!("Token验证失败: {}", e);
            Err((actix_web::error::ErrorUnauthorized("Invalid token"), req))
        }
    }
}