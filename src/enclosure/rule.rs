use std::ffi::OsStr;
use std::path::Path;

use color_eyre::Result;
use log::*;
use serde::{Deserialize, Serialize};

use super::fs::FsDriver;

/// Container for deserialisation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    /// The name of this rule
    pub name: String,
    /// The target directory/file of this rule, ie the path that will be
    /// shadowed.
    pub target: String,
    /// The path to shadow the target with.
    pub rewrite: String,
    /// The mode of the rule, ie whether the target is a file or a directory.
    #[serde(default = "default_rule_mode")]
    pub mode: RuleMode,
    /// The context of the rule, ie the full path to the directories where this rule applies.
    #[serde(default = "empty_vec")]
    pub context: Vec<String>,
    /// The binaries that this rule applies to. If this is not specified, or if
    /// this is an empty list, then the rule applies to all binaries.
    #[serde(default = "empty_vec")]
    pub only: Vec<String>,
}

impl Rule {
    pub fn currently_in_context(&self, fs: &FsDriver) -> Result<bool> {
        if self.context.is_empty() {
            return Ok(true);
        }

        for context in &self.context {
            debug!("{}: resolving context: {}", self.name, context);
            let expanded_context = shellexpand::tilde(&context).to_string();
            let expanded_context = Path::new(&expanded_context).canonicalize()?;
            let resolved_context = fs.maybe_resolve_symlink(&expanded_context)?;

            let pwd = std::env::current_dir()?;

            debug!(
                "{}: {} <> {}",
                self.name,
                pwd.display(),
                resolved_context.display()
            );

            if pwd.starts_with(&resolved_context) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn applies_to_binary(&self, program: &OsStr, fs: &FsDriver) -> Result<bool> {
        if self.only.is_empty() {
            return Ok(true);
        }

        for binary in &self.only {
            debug!("{}: resolving binary: {}", self.name, binary);
            let expanded_binary = fs.fully_expand_path(binary)?;
            let resolved_binary = fs.maybe_resolve_symlink(&expanded_binary)?;

            if program == resolved_binary.file_name().unwrap() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

fn default_rule_mode() -> RuleMode {
    RuleMode::Directory
}

fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleMode {
    File,
    Directory,
}
