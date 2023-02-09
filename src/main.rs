use std::fs;
use std::process::Command;

use atty::Stream;
use clap::Parser;
use color_eyre::Result;
use config::{Config, FileFormat};
use log::*;

use crate::enclosure::rule::Rules;

pub mod enclosure;

#[derive(Parser)]
pub struct Args {
    #[arg(short = 'i', long = "immutable", default_value = "false")]
    pub immutable_root: bool,
    #[arg(trailing_var_arg = true)]
    pub command_with_args: Vec<String>,
    #[arg(short = 'l', long = "log-level", default_value = "info")]
    pub log_level: String,
    #[arg(long = "force-colour", default_value = "false")]
    pub force_colour: bool,
}

fn main() -> Result<()> {
    // Fetch command to run
    let cfg = Args::parse();
    let self_exe = std::env::args().next().unwrap();
    setup_logging(&cfg, &self_exe)?;

    // Load rules
    let rules = load_rules(&self_exe)?;
    info!("loaded {} rules", rules.rules.len());

    // Do the do!
    let (cmd, args) = (&cfg.command_with_args[0], &cfg.command_with_args[1..]);
    let mut command = Command::new(cmd);

    // Pass through current env
    command.envs(std::env::vars());

    // Pass args
    if !args.is_empty() {
        command.args(args);
    }

    // Do the thing!
    enclosure::Enclosure::new(enclosure::Opts {
        rules,
        command: &mut command,
        immutable_root: cfg.immutable_root,
    })
    .run()?;

    Ok(())
}

fn setup_logging(cfg: &Args, self_exe: &str) -> Result<()> {
    if self_exe.starts_with("target/debug") {
        // If no debug set up, basic debugging in dev
        if std::env::var("RUST_DEBUG").is_err() {
            std::env::set_var("RUST_DEBUG", "1");
        }
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "debug");
        }
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", &cfg.log_level);
    }

    if atty::isnt(Stream::Stdin) && !cfg.force_colour {
        // Disable user-friendliness if we're not outputting to a terminal.
        std::env::set_var("NO_COLOR", "1");
        std::env::set_var("RUST_LOG", "warn");
        std::env::remove_var("RUST_DEBUG");
    }

    // Set up basics
    color_eyre::install()?;
    pretty_env_logger::init();

    Ok(())
}

fn load_rules(self_exe: &str) -> Result<Rules> {
    // Set up config file
    let config_file = if self_exe.starts_with("target/debug") {
        "boxxy-dev.yaml"
    } else {
        "boxxy.yaml"
    };
    debug!("loading config: {}", config_file);
    let config_path =
        crate::enclosure::fs::append_all(&dirs::config_dir().unwrap(), vec!["boxxy", config_file]);
    fs::create_dir_all(config_path.parent().unwrap())?;
    if !config_path.exists() {
        info!("no config file found!");
        fs::write(&config_path, "rules: []")?;
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
        Rules { rules: vec![] }
    };

    Ok(rules)
}
