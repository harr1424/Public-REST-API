/*
Stage 1: AI Transcription (Spanish)

Stage 2: Audio Proofreading (Final Transcription in Spanish)

Stage 3: General Translation (Only the parts that are well understood - Bilingual Person)

Stage 4: General Proofreading (Proofreading by native speaker)

Stage 5: Adaptation (Special phrases and literal idioms - Bilingual and Native Group)

Stage 6: Voice Search (Project Coordinator)

Stage 7: Recording (Native Persons)

Stage 8: English Editing (Separate file assembly - Host and Interviewee)

Stage 9: Final Editing (Bilingual Editor)
 */
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, patch, post, HttpResponse};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Stage {
    Any,
    AITranscription,
    AudioProofreading,
    GeneralTranslation,
    GeneralProofreading,
    Adaptation,
    VoiceSearch,
    Recording,
    EnglishEditing,
    FinalEditing,
}

// this should probably be moved to either client side code or else Go http server
#[allow(dead_code)]
fn get_stage(stage: Stage) -> &'static str {
    match stage {
        Stage::AITranscription => "Stage 1: AI Transcription (Spanish)",
        Stage::AudioProofreading => "Stage 2: Audio Proofreading (Final Transcription in Spanish)",
        Stage::GeneralTranslation => "Stage 3: General Translation (Only the parts that are well understood - Bilingual Person)",
        Stage::GeneralProofreading => "Stage 4: General Proofreading (Proofreading by native speaker)",
        Stage::Adaptation => "Stage 5: Adaptation (Special phrases and literal idioms - Bilingual and Native Group)",
        Stage::VoiceSearch => "Stage 6: Voice Search (Project Coordinator)",
        Stage::Recording => "Stage 7: Recording (Native Persons)",
        Stage::EnglishEditing => "Stage 8: English Editing (Separate file assembly - Host and Interviewee)",
        Stage::FinalEditing => "Stage 9: Final Editing (Bilingual Editor)",
        Stage::Any => "All Stages",
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Translation {
    pub id: u32,
    pub name: String,
    pub stage: Stage,
    pub translators: Vec<String>,
    pub due_date: String,
    pub file_url: String,
    pub last_update_by: String,
}

impl Translation {
    fn validate(&self) -> Result<(), String> {
        NaiveDate::parse_from_str(&self.due_date, "%Y-%m-%d").map_err(|_| {
            format!(
                "Invalid date format: {}. Expected format: YYYY-MM-DD",
                self.due_date
            )
        })?;

        Ok(())
    }

    fn clean(&self) -> Self {
        Self {
            id: self.id,
            name: ammonia::clean(&self.name),
            stage: self.stage.clone(),
            translators: self.translators.clone(), // sanitized in translators.rs
            due_date: ammonia::clean(&self.due_date),
            file_url: self.file_url.clone(),
            // TODO add logic to limit URLs to s3 links inside a specific bucket
            last_update_by: ammonia::clean(&self.last_update_by),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Query {
    pub id: Option<u32>,
    pub name: Option<String>,
    pub stage: Option<Stage>,
    pub translators: Option<Vec<String>>,
}

#[post("/translations")]
pub async fn create_translation(
    repo: Data<Arc<Mutex<Vec<Translation>>>>,
    body: Json<Translation>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }
    let mut translation = body.into_inner().clean();

    let mut repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;

    let count: u32 = repo_guard.len() as u32;
    translation.id = count + 1;

    repo_guard.push(translation);

    Ok(HttpResponse::Created().finish())
}

#[get("/translations")]
pub async fn get_translations(
    repo: Data<Arc<Mutex<Vec<Translation>>>>,
    body: Json<Query>,
) -> Result<HttpResponse, actix_web::Error> {
    let repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;

    let mut translations: Vec<Translation> = repo_guard
        .iter()
        .filter(|x| {
            body.id.map_or(true, |q_id| x.id == q_id)
                && body
                    .name
                    .as_ref()
                    .map_or(true, |q_name| x.name.contains(q_name))
                && body.stage.as_ref().map_or(true, |q_stage| {
                    matches!(q_stage, Stage::Any) || &x.stage == q_stage
                })
                && body.translators.as_ref().map_or(true, |q_translators| {
                    q_translators.iter().any(|t| x.translators.contains(t))
                })
        })
        .cloned()
        .collect();
    translations.sort_by(|a, b| a.due_date.cmp(&b.due_date));

    Ok(HttpResponse::Ok()
        .content_type("application/json; charset=utf-8")
        .json(translations))
}

#[patch("/translations")]
pub async fn update_translation(
    // Client is expected to send all updates in payload, payload should be a complete translation object with the last_updated_by reflecting the editor
    repo: Data<Arc<Mutex<Vec<Translation>>>>,
    body: Json<Translation>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }
    let edit = body.into_inner().clean();

    let mut repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;
    if let Some(target) = repo_guard.iter_mut().find(|x| x.id == edit.id) {
        // would a deletion and insertion eb more appropriate here? The payload describes a complete object 
        target.name = edit.name;
        target.stage = edit.stage;
        target.translators = edit.translators;
        target.due_date = edit.due_date;
        target.file_url = edit.file_url;
        target.last_update_by = edit.last_update_by;
    } else {
        return Ok(HttpResponse::NotFound().finish());
    }

    Ok(HttpResponse::Ok().finish())
}

#[delete("/translations/{id}")]
pub async fn delete_translation(
    repo: Data<Arc<Mutex<Vec<Translation>>>>,
    path: Path<u32>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;
    if let Some(position) = repo_guard.iter().position(|x| x.id == path.clone()) {
        repo_guard.remove(position);
    } else {
        return Ok(HttpResponse::NotFound().finish());
    }

    Ok(HttpResponse::Ok().finish())
}
