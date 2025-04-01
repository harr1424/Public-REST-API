use crate::translations::*;
use crate::translators::*;
use crate::{api::*, hosts::*, instructors::*};
use actix_web::web::ServiceConfig;

pub fn config_eng_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_eng);
    cfg.service(get_engs);
    cfg.service(edit_eng);
    cfg.service(delete_eng);
}

pub fn config_translation_paths(cfg: &mut ServiceConfig) {
    cfg.service(create_translation);
    cfg.service(get_translations);
    cfg.service(update_translation);
    cfg.service(delete_translation);
}

pub fn config_ins_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_instructor);
    cfg.service(get_instructors);
    cfg.service(delete_instructor);
}

pub fn config_hosts_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_host);
    cfg.service(get_hosts);
    cfg.service(delete_host);
}

pub fn config_translators_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_translator);
    cfg.service(get_translators);
    cfg.service(delete_translator);
}
