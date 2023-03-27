use std::fs;
use std::path::PathBuf;
use std::process::Command;

use atty::Stream;
use clap::{Parser, Subcommand};
use color_eyre::Result;
use config::{Config, FileFormat};
use log::*;
use scanner::App;
use which::which;

use crate::enclosure::rule::{BoxxyConfig, Rule, RuleMode};
use crate::scanner::Scanner;

pub mod enclosure;
pub mod scanner;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "boxxy",
    display_name = "boxxy",
    about = "Put bad programs in a box with only their files.",
    long_about = "boxxy forces bad programs to put their files somewhere else via Linux user namespaces.",
    version = VERSION,
    subcommand_negates_reqs = true,
)]
pub struct Args {
    #[arg(
        short = 'i',
        long = "immutable",
        default_value = "false",
        help = "Make the root filesystem immutable."
    )]
    pub immutable_root: bool,

    #[arg(
        trailing_var_arg = true,
        name = "COMMAND TO RUN",
        required = true,
        help = "The command to run, ex. `ls -lah` or `aws configure`."
    )]
    pub command_with_args: Vec<String>,

    #[arg(short = 'l', long = "log-level", default_value = "info")]
    pub log_level: String,

    #[arg(
        long = "force-colour",
        default_value = "false",
        help = "Force colour output even when stdout is not a tty."
    )]
    pub force_colour: bool,

    #[arg(
        short = 't',
        long = "trace",
        default_value = "false",
        help = "Enable tracing of I/O-related syscalls and generate a report of files/directories the program touched."
    )]
    pub trace: bool,

    #[command(subcommand)]
    pub command: Option<BoxxySubcommand>,
}

#[derive(Subcommand)]
pub enum BoxxySubcommand {
    #[command(
        name = "config",
        about = "View the config file.",
        subcommand_negates_reqs = true,
        aliases = &["cfg", "conf", "c"]
    )]
    Config,
    #[command(
        name = "scan",
        about = "Scan your homedir for applications that may benefit from boxxy.",
        subcommand_negates_reqs = true,
        aliases = &["s"]
    )]
    Scan,
}

fn main() -> Result<()> {
    // Fetch command to run
    let cfg = Args::parse();
    let self_exe = std::env::args().next().unwrap();
    setup_logging(&cfg, &self_exe)?;

    if let Some(cmd) = cfg.command {
        match cmd {
            BoxxySubcommand::Config => {
                let config_path = config_file_path(&self_exe)?;
                let mut printer = bat::PrettyPrinter::new();
                printer.input_file(config_path).print()?;

                return Ok(());
            }
            BoxxySubcommand::Scan => {
                let apps = Scanner::new().scan()?;
                return scan_homedir(&self_exe, apps);
            }
        }
    }

    // Load rules
    let rules = load_rules(&self_exe)?;
    info!("loaded {} rule(s)", rules.rules.len());

    // Do the do!
    let (cmd, args) = (&cfg.command_with_args[0], &cfg.command_with_args[1..]);

    if which(cmd).is_err() {
        error!("command not found in $PATH: {}", cmd);
        debug!("searched $PATH: {}", std::env::var("PATH")?);
        std::process::exit(1);
    }

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
        trace: cfg.trace,
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

    if atty::isnt(Stream::Stdout) && !cfg.force_colour {
        // Disable user-friendliness if we're not outputting to a terminal.
        std::env::set_var("NO_COLOR", "1");
        std::env::set_var("RUST_LOG", "warn");
        std::env::remove_var("RUST_DEBUG");
    }

    // Set up basics
    color_eyre::config::HookBuilder::new()
        .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .install()?;

    pretty_env_logger::init();

    Ok(())
}

fn config_file_path(self_exe: &str) -> Result<PathBuf> {
    let config_file = if self_exe.starts_with("target/debug") {
        "boxxy-dev.yaml"
    } else {
        "boxxy.yaml"
    };

    debug!("loading config: {}", config_file);

    let config_path =
        enclosure::fs::append_all(&dirs::config_dir().unwrap(), vec!["boxxy", config_file]);

    fs::create_dir_all(config_path.parent().unwrap())?;
    if !config_path.exists() {
        info!("no config file found!");
        fs::write(&config_path, "rules: []")?;
        info!("created empty config at {}", config_path.display());
    }

    Ok(config_path)
}

fn load_rules(self_exe: &str) -> Result<BoxxyConfig> {
    let config_path = config_file_path(self_exe)?;
    let rules = if fs::metadata(&config_path)?.len() > 0 {
        let config = Config::builder()
            .add_source(config::File::new(
                &config_path.as_path().to_string_lossy(),
                FileFormat::Yaml,
            ))
            .build()?;
        config.try_deserialize::<BoxxyConfig>()?
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
        BoxxyConfig { rules: vec![] }
    };

    Ok(rules)
}

fn scan_homedir(self_exe: &str, apps: Vec<App>) -> Result<()> {
    if !apps.is_empty() {
        info!(
            "found {} applications that might be boxxable! generating config...",
            apps.len()
        );
        let mut rules = vec![];
        for app in apps {
            for fix in app.fixes {
                let (old, new) = fix.split_once(':').unwrap();
                let path = PathBuf::from(old);
                let mode = if path.is_dir() {
                    RuleMode::Directory
                } else {
                    RuleMode::File
                };
                rules.push(Rule {
                    name: app.name.clone(),
                    target: old.into(),
                    rewrite: new.into(),
                    mode,
                    context: vec![],
                    only: vec![],
                });
            }
        }
        let config = BoxxyConfig {
            rules: rules.clone(),
        };
        let config = &serde_yaml::to_string(&config)?;
        let mut printer = bat::PrettyPrinter::new();
        println!();
        printer
            .input_from_bytes(config.as_bytes())
            .language("yaml")
            .print()
            .expect("failed to print config");
        println!();
        warn!("!!! BE CAREFUL WITH THIS CONFIG !!!");
        warn!("SAFETY IS NOT GUARANTEED!!!");
        warn!("this config was automatically generated and may not be correct.");
        warn!("please review the config before using it!");
        warn!("report bad rules!! https://github.com/queer/boxxy/issues/new");
        info!("rules generated: {}", rules.len());
        info!(
            "put relevant rules in your config file: {}",
            config_file_path(self_exe)?.display()
        );
    }

    Ok(())
}
