use std::path::PathBuf;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct App {
    pub name: String,
    pub paths: Vec<String>,
    pub fixes: Vec<String>,
}

pub struct Scanner {
    pub apps: Vec<App>,
}

const HARDCODED_APPS_JSON: &str = include_str!("../../data/hardcoded-applications.json");
const PARTIAL_APPS_JSON: &str = include_str!("../../data/partial-support-applications.json");

impl Scanner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut hardcoded = serde_json::from_str::<Vec<App>>(HARDCODED_APPS_JSON).unwrap();
        let mut partial = serde_json::from_str::<Vec<App>>(PARTIAL_APPS_JSON).unwrap();
        let mut apps = vec![];
        apps.append(&mut hardcoded);
        apps.append(&mut partial);

        Self { apps }
    }

    pub fn scan(&mut self) -> Result<Vec<App>> {
        let mut out = vec![];

        for app in &self.apps {
            for path in &app.paths {
                let path = shellexpand::full(&path)?.to_string();
                if PathBuf::from(path).exists() {
                    out.push(app.clone());
                }
            }
        }

        Ok(out)
    }
}
