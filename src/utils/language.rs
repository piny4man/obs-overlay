use std::{
    fs::{self},
    path::Path,
};

use serde_json::{from_str, json};

pub fn get_language_color(language: &str) -> String {
    let file = fs::read_to_string(Path::new("colors.json")).expect("Failed to read colors.json");
    let colors = from_str::<serde_json::Value>(&file);

    match colors {
        Ok(colors) => match colors.get(language) {
            Some(color) => color.to_string(),
            None => json!("").to_string(),
        },
        Err(_) => json!("").to_string(),
    }
}

pub fn get_language_size(language: &u64, total: &u64) -> f64 {
    (*language as f64 / *total as f64) * 100.0
}
