use crate::TranslatorRepo;
use actix_web::{
    delete, get, post,
    web::{Data, Path},
    HttpResponse,
};
#[post("/translators/{new}")]
pub async fn add_translator(
    repo: Data<TranslatorRepo>,
    new: Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let sanitized = ammonia::clean(&new);

    match repo.lock() {
        Ok(mut repo) => {
            repo.insert(sanitized);
            Ok(HttpResponse::Created().finish())
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[get("/translators")]
pub async fn get_translators(repo: Data<TranslatorRepo>) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(repo) => {
            let mut translators: Vec<String> = repo.iter().cloned().collect();
            translators.sort();

            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .json(translators))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[delete("/translators/{i}")]
pub async fn delete_translator(
    repo: Data<TranslatorRepo>,
    i: Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let sanitized = ammonia::clean(&i);

    match repo.lock() {
        Ok(mut repo) => {
            if repo.contains(&sanitized) {
                repo.remove(&sanitized);
                Ok(HttpResponse::Ok().finish())
            } else {
                Ok(HttpResponse::NotFound().finish())
            }
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}
