use actix_web::{
    delete, get, patch, post,
    web::{Data, Json, Path},
    HttpResponse,
};
use chrono::NaiveDate;
use serde_json::json;
use std::cmp::Ordering;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Language {
    Any,
    English,
    Spanish,
    French,
    Portuguese,
    Italian,
    German,
    Persian,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Status {
    Planning,
    Invited,
    Confirmed,
    Rejected,
    Complete,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum HostStatus {
    Planning,
    Invited,
    Confirmed,
    Rejected,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum FlyerStatus {
    Pending,
    Sent,
    Complete,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct EngagementQuery {
    pub language: Option<Language>,
    pub number: Option<String>,
    pub activity_type: Option<String>,
    pub instructor: Option<String>,
    pub host: Option<String>,
    pub date: Option<String>,
    pub status: Option<Status>,
    pub host_status: Option<HostStatus>,
    pub flyer_status: Option<FlyerStatus>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Engagement {
    pub id: Uuid,
    pub instructor: String,
    pub host: String,
    pub date: String,
    pub language: Language,
    pub title: String,
    pub part: usize,
    pub num_parts: usize,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_status: Option<HostStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flyer_status: Option<FlyerStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated_by: Option<String>,
}

impl Engagement {
    fn validate(&self) -> Result<(), String> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|_| {
            format!(
                "Invalid date format: {}. Expected format: YYYY-MM-DD",
                self.date
            )
        })?;

        if self.part == 0 {
            return Err("Part number must be greater than 0".to_string());
        }

        if self.num_parts == 0 {
            return Err("Number of parts must be greater than 0".to_string());
        }

        if self.part > self.num_parts {
            return Err(format!(
                "Part number ({}) cannot be greater than total number of parts ({})",
                self.part, self.num_parts
            ));
        }

        Ok(())
    }

    fn clean(&self) -> Self {
        Self {
            id: self.id,
            instructor: ammonia::clean(&self.instructor),
            host: ammonia::clean(&self.host),
            date: ammonia::clean(&self.date),
            language: self.language.clone(),
            title: ammonia::clean(&self.title),
            part: self.part,
            num_parts: self.num_parts,
            status: self.status.clone(),
            host_status: self.host_status.clone(),
            flyer_status: self.flyer_status.clone(),
            notes: Some(ammonia::clean(
                self.notes.clone().unwrap_or(String::new()).as_str(),
            )),
            number: Some(ammonia::clean(
                self.number.clone().unwrap_or_default().as_str(),
            )),
            activity_type: Some(ammonia::clean(
                self.activity_type.clone().unwrap_or_default().as_str(),
            )),
            last_updated_by: self.last_updated_by.clone(),
        }
    }
}

impl std::hash::Hash for Engagement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Engagement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Engagement {}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct NewEngagement {
    pub instructor: String,
    pub host: String,
    pub date: String,
    pub language: Language,
    pub title: String,
    pub part: usize,
    pub num_parts: usize,
    pub status: Status,
    pub host_status: HostStatus,
    pub flyer_status: FlyerStatus,
    pub notes: String,
    pub number: String,
    pub activity_type: String,
    pub last_updated_by: String,
}

impl NewEngagement {
    fn validate(&self) -> Result<(), String> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|_| {
            format!(
                "Invalid date format: {}. Expected format: YYYY-MM-DD",
                self.date
            )
        })?;

        if self.part == 0 {
            return Err("Part number must be greater than 0".to_string());
        }

        if self.num_parts == 0 {
            return Err("Number of parts must be greater than 0".to_string());
        }

        if self.part > self.num_parts {
            return Err(format!(
                "Part number ({}) cannot be greater than total number of parts ({})",
                self.part, self.num_parts
            ));
        }

        Ok(())
    }
}

#[post("/engs")]
pub async fn add_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<NewEngagement>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }

    // Create the new engagement outside the lock
    let new_eng = Engagement {
        id: Uuid::new_v4(),
        instructor: ammonia::clean(&body.instructor),
        host: ammonia::clean(&body.host),
        date: ammonia::clean(&body.date),
        language: body.language.clone(),
        title: ammonia::clean(&body.title),
        part: body.part,
        num_parts: body.num_parts,
        status: body.status.clone(),
        host_status: Some(body.host_status.clone()),
        flyer_status: Some(body.flyer_status.clone()),
        notes: Some(ammonia::clean(&body.notes)),
        number: Some(ammonia::clean(&body.number)),
        activity_type: Some(body.activity_type.clone()),
        last_updated_by: Some(format!(
            "{}  {}",
            body.last_updated_by.clone(),
            chrono::Utc::now().format("%Y-%m-%d")
        )),
    };

    let mut repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;

    // Check if number exists and collect engagements to update
    let num = body.number.parse::<usize>().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Invalid number format: {}", e))
    })?;

    let existing_numbers: Vec<_> = repo_guard
        .iter()
        .filter_map(|eng| {
            eng.number
                .as_ref()
                .and_then(|n| n.parse::<usize>().ok())
                .map(|n| (eng.clone(), n))
        })
        .collect();

    let number_exists = existing_numbers.iter().any(|(_, n)| *n == num);

    if number_exists {
        // Update numbers in a single pass
        let to_update: Vec<_> = existing_numbers
            .into_iter()
            .filter(|(_, existing)| *existing >= num)
            .map(|(eng, _)| eng)
            .collect();

        for eng in to_update {
            repo_guard.remove(&eng);
            let mut updated = eng;
            if let Some(ref mut curr_num) = updated.number {
                if let Ok(existing_num) = curr_num.parse::<usize>() {
                    *curr_num = (existing_num + 1).to_string();
                }
            }
            repo_guard.insert(updated);
        }
    }

    repo_guard.insert(new_eng);

    Ok(HttpResponse::Created().finish())
}

#[get("/engs")]
pub async fn get_engs(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<EngagementQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(repo) => {
            let mut engagements: Vec<Engagement> = repo
                .iter()
                .filter(|x| {
                    body.language.as_ref().map_or(true, |lang| {
                        matches!(lang, Language::Any) || x.language == *lang
                    }) && body.number.as_ref().map_or(true, |q_num| {
                        x.number.as_ref().map_or(false, |x_num| x_num == q_num)
                    }) && body.activity_type.as_ref().map_or(true, |q_act| {
                        x.activity_type
                            .as_ref()
                            .map_or(false, |x_act| x_act == q_act)
                    }) && body
                        .instructor
                        .as_ref()
                        .map_or(true, |q_inst| x.instructor == *q_inst)
                        && body.host.as_ref().map_or(true, |q_host| x.host == *q_host)
                        && body.date.as_ref().map_or(true, |q_date| x.date == *q_date)
                        && body
                            .status
                            .as_ref()
                            .map_or(true, |q_status| x.status == *q_status)
                        && body.host_status.as_ref().map_or(true, |q_host_status| {
                            x.host_status
                                .as_ref()
                                .map_or(false, |x_host_status| x_host_status == q_host_status)
                        })
                        && body.flyer_status.as_ref().map_or(true, |q_flyer_status| {
                            x.flyer_status
                                .as_ref()
                                .map_or(false, |x_flyer_status| x_flyer_status == q_flyer_status)
                        })
                })
                .cloned()
                .collect();

            engagements.sort_by(|a, b| {
                match (a.number.as_ref(), b.number.as_ref()) {
                    (Some(num_a), Some(num_b)) => {
                        // Both have numbers, compare them.  Handle potential parsing errors.
                        let num_a_parsed = num_a.parse::<usize>();
                        let num_b_parsed = num_b.parse::<usize>();

                        match (num_a_parsed, num_b_parsed) {
                            (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num), // Compare parsed numbers
                            (Ok(_), Err(_)) => Ordering::Less, // a is a valid number, b is not - a comes first
                            (Err(_), Ok(_)) => Ordering::Greater, // b is a valid number, a is not - b comes first
                            (Err(_), Err(_)) => num_a.cmp(num_b), // Both are invalid numbers, compare as strings
                        }
                    }
                    (Some(_), None) => Ordering::Less, // a has a number, b doesn't - a comes first
                    (None, Some(_)) => Ordering::Greater, // b has a number, a doesn't - b comes first
                    (None, None) => a.date.cmp(&b.date),  // Neither has a number, compare by date
                }
            });

            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .json(engagements))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock (GET)",
        )),
    }
}

#[patch("/engs")]
pub async fn edit_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<Engagement>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }

    match repo.lock() {
        Ok(mut repo) => {
            let mut target_eng = body.into_inner().clean();
            let update_string = target_eng.last_updated_by.clone();
            target_eng.last_updated_by = Some(format!(
                "{} {}",
                update_string.unwrap_or_default(),
                chrono::Utc::now().format("%Y-%m-%d")
            ));
            if repo.contains(&target_eng) {
                repo.remove(&target_eng);
                repo.insert(target_eng.clone());
                Ok(HttpResponse::Ok().finish())
            } else {
                Ok(HttpResponse::NotFound().finish())
            }
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock (UPDATE)",
        )),
    }
}

#[delete("/engs/{id}")]
pub async fn delete_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    path: Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let target_id = path.into_inner();

    let mut repo_guard = repo
        .lock()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to acquire repo lock"))?;

    let target_eng = repo_guard.iter().find(|e| e.id == target_id).cloned();

    if let Some(eng) = target_eng {
        repo_guard.remove(&eng);

        // If it has a number, process the decrements
        if let Some(num) = eng.number {
            let parsed_num = num
                .parse::<usize>()
                .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid number format"))?;

            // Create vector of engagements to update
            let to_update: Vec<_> = repo_guard
                .iter()
                .filter(|e| {
                    e.number
                        .as_ref()
                        .and_then(|n| n.parse::<usize>().ok())
                        .map_or(false, |existing| existing > parsed_num)
                })
                .cloned()
                .collect();

            // Remove all affected engagements
            for eng in &to_update {
                repo_guard.remove(eng);
            }

            // Insert updated engagements
            for mut update_eng in to_update {
                if let Some(ref mut curr_num) = update_eng.number {
                    if let Ok(existing_num) = curr_num.parse::<usize>() {
                        *curr_num = (existing_num - 1).to_string();
                    }
                }
                repo_guard.insert(update_eng);
            }
        }

        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
