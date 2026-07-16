use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use steel::steel_vm::{builtin::BuiltInModule, engine::Engine, register_fn::RegisterFn};

use crate::app::tile::ExtSender;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct ExtensionItem {
    src_file: PathBuf,
    name: String,
    icon_source_file: PathBuf,
    ext_type: ExtensionType,
    ext_config: HashMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionType {
    Constant,
    Polled(Duration),
    Dynamic,
}

pub struct Action {
    run_file: PathBuf,
}

pub struct ExtensionEngine {
    engine: Arc<Mutex<Engine>>,
    extensions: Vec<ExtensionItem>,
}

impl ExtensionEngine {
    pub fn new(sender: ExtSender) -> Self {
        let mut engine = Engine::new();
        let mut module = BuiltInModule::new("rustcast/core");
        let sender = sender.clone();

        module.register_fn("cli", extension_prelude::call_command);
        module.register_fn("fswrite", extension_prelude::create_file);
        module.register_fn("fsread", extension_prelude::read_file);
        module.register_fn(
            "populate",
            move |name: String, search: String, icon_path: String| {
                extension_prelude::populate(sender.clone(), name, search, icon_path)
            },
        );

        engine.register_module(module);

        ExtensionEngine {
            engine: Arc::new(Mutex::new(engine)),
            extensions: vec![],
        }
    }

    pub fn load_extension(&mut self, extension_config_path: PathBuf) {
        let Ok(config_str) = fs::read_to_string(extension_config_path) else {
            return;
        };

        if let Some(conf) = toml::from_str(&config_str).ok() {
            self.extensions.push(conf);
        }
    }
}

mod extension_prelude {
    use std::path::Path;

    use iced::futures::SinkExt;
    use log::info;

    use crate::app::tile::ExtSender;
    pub fn call_command(command: String) -> String {
        info!("Extension is calling CLI: {command}");
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .ok()
            .and_then(|x| String::from_utf8(x.stdout).ok())
            .unwrap_or_default()
    }

    pub fn create_file(path: String, contents: String) {
        info!("Extension called file write function");
        std::fs::write(path, contents).ok();
    }

    pub fn read_file(path: String) -> String {
        info!("Extension called file write function");
        std::fs::read_to_string(path).unwrap_or_default()
    }

    pub fn populate(sender: ExtSender, name: String, search_name: String, icon_path: String) {
        let sender = sender.clone();
        tokio::task::spawn_blocking(async move || {
            let icon_path = icon_path.clone();
            let mut sender = sender.clone();
            sender
                .0
                .send(crate::app::Message::AddExtensionApp {
                    display: name,
                    search: search_name,
                    icon_path: Path::new(&(icon_path.clone())).to_path_buf(),
                })
                .await
                .unwrap();
        });
    }
}
