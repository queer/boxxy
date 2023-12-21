use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::Result;
use log::*;

use crate::enclosure::rule::{BoxxyRules, Rule};

pub struct BoxxyConfig {
    pub rules: BoxxyRules,
    pub immutable_root: bool,
    pub trace: bool,
    pub dotenv: bool,
    pub daemon: bool,
    pub command: Command,
}

impl BoxxyConfig {
    pub fn debug_mode() -> Result<bool> {
        let self_exe = std::fs::read_link("/proc/self/exe")?;
        Ok(self_exe
            .into_os_string()
            .to_string_lossy()
            .contains("target/debug"))
    }

    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().unwrap();
        Ok(crate::enclosure::fs::append_all(
            &config_dir,
            vec!["boxxy", Self::default_config_file_name()?],
        ))
    }

    pub fn default_config_file_name() -> Result<&'static str> {
        if Self::debug_mode()? {
            Ok("boxxy-dev.yaml")
        } else {
            Ok("boxxy.yaml")
        }
    }

    pub fn rule_paths() -> Result<Vec<PathBuf>> {
        let config_file_name = Self::default_config_file_name()?;

        let default_config_file = {
            let config_dir = dirs::config_dir().unwrap();
            let config_path =
                crate::enclosure::fs::append_all(&config_dir, vec!["boxxy", config_file_name]);

            std::fs::create_dir_all(config_path.parent().unwrap())?;

            config_path
        };

        let mut config_paths = vec![];
        if default_config_file.exists() {
            config_paths.push(default_config_file);
        }

        // Search up the tree for a `config_file_name` file
        let mut current_dir = std::env::current_dir()?;
        debug!(
            "searching for boxxy config starting at {}",
            current_dir.display()
        );
        loop {
            let config_path =
                crate::enclosure::fs::append_all(&current_dir, vec![config_file_name]);
            debug!("checking for: {}", config_path.display());
            if config_path.exists() {
                debug!("found boxxy config file at {}", config_path.display());
                config_paths.push(config_path);
            }

            if let Some(parent) = current_dir.parent() {
                if parent == current_dir {
                    debug!("ran out of parents to search!");
                    break;
                }
                current_dir = parent.to_path_buf();
            } else {
                debug!("ran out of parents to search!");
                break;
            }
        }

        Ok(config_paths)
    }

    pub fn load_rules_from_path(path: &Path) -> Result<BoxxyRules> {
        let config = config::Config::builder()
            .add_source(config::File::new(
                &path.to_string_lossy(),
                config::FileFormat::Yaml,
            ))
            .build()?;

        let rules = config.try_deserialize::<BoxxyRules>()?;

        Ok(rules)
    }

    pub fn load_rules_from_cli_flag(rules: &[String]) -> Result<BoxxyRules> {
        let rules = rules
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.split(':').collect();
                match parts.as_slice() {
                    [src, dest] => Rule {
                        name: format!("cli-loaded rule: {src} -> {dest}"),
                        target: src.to_string(),
                        rewrite: dest.to_string(),
                        mode: crate::enclosure::rule::RuleMode::File,
                        context: vec![],
                        only: vec![],
                        env: HashMap::new(),
                    },

                    [src, dest, mode] => Rule {
                        name: format!("cli-loaded rule: {src} -> {dest} ({mode})"),
                        target: src.to_string(),
                        rewrite: dest.to_string(),
                        mode: mode.parse().unwrap(),
                        context: vec![],
                        only: vec![],
                        env: HashMap::new(),
                    },

                    _ => panic!("invalid format for cli rule: {s}"),
                }
            })
            .collect();
        Ok(BoxxyRules { rules })
    }

    pub fn merge(configs: Vec<BoxxyRules>) -> BoxxyRules {
        let mut merged = BoxxyRules { rules: vec![] };
        for config in configs {
            merged.rules.extend(config.rules);
        }

        merged
    }

    pub fn load_config(args: crate::Args) -> Result<Self> {
        // Load rules
        let rules = {
            let mut rules = vec![];
            if !args.no_config {
                debug!("loading rules (not asked not to!)");
                for config in BoxxyConfig::rule_paths()? {
                    info!("loading rules from {}", config.display());
                    rules.push(BoxxyConfig::load_rules_from_path(&config)?);
                }
            }
            rules.push(BoxxyConfig::load_rules_from_cli_flag(&args.arg_rules)?);
            BoxxyConfig::merge(rules)
        };
        info!("loaded {} total rule(s)", rules.rules.len());

        let (cmd, cmd_args) = (&args.command_with_args[0], &args.command_with_args[1..]);

        if which::which(cmd).is_err() {
            // If `which` can't find it, check if the path exists.
            if !Path::new(cmd).exists() {
                error!("command not found in $PATH or by path: {}", cmd);
                debug!("searched $PATH: {}", std::env::var("PATH")?);
                std::process::exit(1);
            }
        }

        let mut command = Command::new(cmd);

        // Pass through current env
        command.envs(std::env::vars());

        // Pass args
        if !cmd_args.is_empty() {
            command.args(cmd_args);
        }

        Ok(Self {
            rules,
            immutable_root: args.immutable_root,
            trace: args.trace,
            dotenv: args.dotenv,
            daemon: args.daemon,
            command,
        })
    }
}
