use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct UserConfig {
    pub log_level: Option<String>,
    pub render: Option<RenderUserConfig>,
    pub startup: Option<UserStartupConfig>,
}

impl UserConfig {
    pub fn get_log_level(&self) -> log::LevelFilter {
        if let Some(s) = &self.log_level {
            match s.to_lowercase().as_str() {
                "trace" => return log::LevelFilter::Trace,
                "debug" => return log::LevelFilter::Debug,
                "info" => return log::LevelFilter::Info,
                "warn" => return log::LevelFilter::Warn,
                "error" => return log::LevelFilter::Error,
                "off" => return log::LevelFilter::Off,
                _ => {
                    log::warn!("Invalid log level '{}' specified in user config.", s);
                }
            }
        }

        log::LevelFilter::Off
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            log_level: Some("warn".to_string()),
            render: None,
            startup: None,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct RenderUserConfig {
    pub renderer_path: Option<String>,
    pub update_frequency: Option<u32>,
}

#[derive(Deserialize, Clone)]
pub struct UserStartupConfig {
    pub startup_window: Option<String>,
}

pub enum StartupWindowOption {
    Startup,
    Render,
}

impl UserStartupConfig {
    pub fn get_startup_window_option(&self) -> StartupWindowOption {
        if let Some(s) = &self.startup_window {
            if s.to_lowercase() == "render" {
                return StartupWindowOption::Render;
            }

            log::warn!("Invalid startup window '{}' specified in user config.", s);
        }

        StartupWindowOption::Startup
    }
}
