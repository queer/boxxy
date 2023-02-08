use std::fs;
use std::process::Command;

use color_eyre::Result;
use config::{Config, FileFormat};
use log::*;

use crate::enclosure::rule::Rules;

pub mod enclosure;

fn main() -> Result<()> {
    // Fetch command to run
    let args = std::env::args().collect::<Vec<String>>();
    let (self_exe, cmd, maybe_args) = {
        #[allow(clippy::comparison_chain)]
        if args.len() == 2 {
            (args[0].as_str(), args[1].as_str(), None)
        } else if args.len() > 2 {
            (args[0].as_str(), args[1].as_str(), Some(&args[2..]))
        } else {
            panic!("Usage: {} <cmd> [args...]", args[0]);
        }
    };

    if self_exe.starts_with("target/debug") {
        // If no debug set up, basic debugging in dev
        if std::env::var("RUST_DEBUG").is_err() {
            std::env::set_var("RUST_DEBUG", "1");
        }
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "debug");
        }
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Set up basics
    color_eyre::install()?;
    pretty_env_logger::init();

    // Set up config file
    let config_file = if self_exe.starts_with("target/debug") {
        "boxxy-dev.yaml"
    } else {
        "boxxy.yaml"
    };
    let config_path =
        crate::enclosure::fs::append_all(&dirs::config_dir().unwrap(), vec!["boxxy", config_file]);
    fs::create_dir_all(config_path.parent().unwrap())?;
    if !config_path.exists() {
        info!("no config file found!");
        fs::write(&config_path, "rules:\n")?;
        info!("created empty config at {}", config_path.display());
    }

    // Load rules from config
    let rules = if fs::metadata(&config_path)?.len() > 0 {
        let config = Config::builder()
            .add_source(config::File::new(
                &config_path.as_path().to_string_lossy(),
                FileFormat::Yaml,
            ))
            .build()?;
        config.try_deserialize::<Rules>()?
    } else {
        warn!("you have no rules in your config file.");
        warn!("try adding some rules to {config_path:?}");
        warn!(
            r#"
example rule:

    rules:
    - name: "make aws cli write to ~/.config/aws"
      target: "~/.aws"
      rewrite: "~/.config/aws"
        "#
        );
        Rules {
            rules: vec![]
        }
    };
    info!("loaded {} rules", rules.rules.len());

    // Do the do!
    let mut command = Command::new(cmd);

    // Pass through current env
    command.envs(std::env::vars());

    // Pass args
    if let Some(args) = maybe_args {
        command.args(args);
    }

    enclosure::Enclosure::new(rules, &mut command).run()?;

    Ok(())
}
