use actix_route_rate_limiter::{LimiterBuilder, RateLimiter};
use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use dotenv::dotenv;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
};

mod api;
mod backup;
mod hosts;
mod instructors;
mod routing;
mod security_headers;
mod translations;
mod translators;
mod types;

use api::Engagement;
use backup::{BackupConfig, BackupSystem};
use security_headers::SecurityHeaders;
use translations::Translation;
use types::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect("Failed to read .env file");
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let listen_addr = env::var("LISTEN_ADDR").expect("LISTEN_HTTP must be set");
    let cert_path = env::var("TLS_CERT_PATH").expect("TLS_CERT_PATH must be set");
    let key_path = env::var("TLS_KEY_PATH").expect("TLS_KEY_PATH must be set");
    let rustls_config = load_rustls_config(&cert_path, &key_path)?;

    let engagements: Arc<Mutex<HashSet<Engagement>>> = Arc::new(Mutex::new(HashSet::new()));
    let instructors = InstructorRepo::new();
    let hosts = HostRepo::new();
    let translations: Arc<Mutex<Vec<Translation>>> = Arc::new(Mutex::new(Vec::new()));
    let translators = TranslatorRepo::new();

    let backup_engagements = engagements.clone();
    let backup_instructors = instructors.clone();
    let backup_hosts = hosts.clone();
    let backup_translations = translations.clone();
    let backup_translators = translators.clone();

    // let load_instructors = instructors.clone();
    // load_instructors_from_file(load_instructors)?; // used once to seed instructors 

    if let Err(e) = configure_backup_system(
        backup_engagements.clone(),
        backup_instructors,
        backup_hosts,
        backup_translations,
        backup_translators,
    )
    .await
    {
        log::error!("Failed to configure backup system: {}", e);
    }

    let limiter = LimiterBuilder::new()
        .with_duration(chrono::Duration::minutes(1))
        .with_num_requests(60)
        .build();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(SecurityHeaders)
            .wrap(RateLimiter::new(Arc::clone(&limiter)))
            .app_data(Data::new(engagements.clone()))
            .app_data(Data::new(instructors.clone()))
            .app_data(Data::new(hosts.clone()))
            .app_data(Data::new(translations.clone()))
            .app_data(Data::new(translators.clone()))
            .service(
                web::scope("")
                    .configure(routing::config_eng_paths)
                    .configure(routing::config_ins_paths)
                    .configure(routing::config_hosts_paths)
                    .configure(routing::config_translation_paths)
                    .configure(routing::config_translators_paths),
            )
    })
    .bind_rustls(&listen_addr, rustls_config)?
    //.bind(&listen_addr)?
    .run()
    .await
}

fn load_rustls_config(cert_path: &str, key_path: &str) -> std::io::Result<ServerConfig> {
    let cert_file = &mut BufReader::new(File::open(cert_path)?);
    let cert_chain = certs(cert_file)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid cert"))?
        .into_iter()
        .map(Certificate)
        .collect();

    let key_file = &mut BufReader::new(File::open(key_path)?);
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key"))?
        .into_iter()
        .map(PrivateKey)
        .collect();

    if keys.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "No private key found",
        ));
    }

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(config)
}

async fn configure_backup_system(
    engagements: Arc<Mutex<HashSet<Engagement>>>,
    instructors: InstructorRepo,
    hosts: HostRepo,
    translations: Arc<Mutex<Vec<Translation>>>,
    translators: TranslatorRepo,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = BackupConfig::from_env()?;
    let backup_system = BackupSystem::new(
        engagements.clone(),
        instructors.0.clone(),
        hosts.0.clone(),
        translations.clone(),
        translators.0.clone(),
        config,
    )
    .await?;

    {
        let (
            mut engagements_guard,
            mut instructors_guard,
            mut hosts_guard,
            mut translations_guard,
            mut translators_guard,
        ) = (
            engagements.lock().unwrap(),
            instructors.lock().unwrap(),
            hosts.lock().unwrap(),
            translations.lock().unwrap(),
            translators.lock().unwrap(),
        );

        if engagements_guard.is_empty()
            || instructors_guard.is_empty()
            || hosts_guard.is_empty()
            || translations_guard.is_empty()
            || translators_guard.is_empty()
        {
            match backup_system.restore_latest_backup().await {
                Ok((
                    restored_engagements,
                    restored_instructors,
                    restored_hosts,
                    restored_translations,
                    restored_translators,
                )) => {
                    if engagements_guard.is_empty() {
                        *engagements_guard = restored_engagements;
                        log::info!("Successfully restored engagements from latest backup");
                    }

                    if hosts_guard.is_empty() {
                        *hosts_guard = restored_hosts;
                        log::info!("Successfully restored hosts from latest backup");
                    }

                    if instructors_guard.is_empty() {
                        *instructors_guard = restored_instructors;
                        log::info!("Successfully restored instructors from latest backup");
                    }

                    if translations_guard.is_empty() {
                        *translations_guard = restored_translations;
                        log::info!("Successfully restored translations from latest backup");
                    }

                    if translators_guard.is_empty() {
                        *translators_guard = restored_translators;
                        log::info!("Successfully restored translators from latest backup");
                    }
                }
                Err(e) => {
                    log::error!("Failed to restore data from backup: {}", e);
                }
            }
        }
    }

    backup_system.start_backup_task().await;

    Ok(())
}

// fn load_instructors_from_file(instructors: InstructorRepo) -> Result<(), std::io::Error> {
//     let file = File::open("m.txt")?;
//     let reader = BufReader::new(&file);

//     let mut line_count = 0;
//     let mut inserts = 0;

//     match instructors.lock() {
//         Ok(mut repo) => {
//             for line in reader.lines() {
//                 line_count += 1;
//                 if let Ok(line) = line {
//                     repo.insert(String::from(line));
//                     inserts += 1;
//                 } else {
//                     log::error!("Unable to insert instructor with line: {:?}", line)
//                 }
//             }
//         }
//         Err(e) => {
//             log::error!("Unable to lock InstructorRepo for file load: {}", e)
//         }
//     }

//     log::info!(
//         "Loaded {} instructors from {} lines in load file",
//         inserts,
//         line_count
//     );

//     Ok(())
// }
