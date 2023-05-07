use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use log::*;
use serde::{Deserialize, Serialize};

use super::fs::FsDriver;

/// Container for deserialisation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoxxyRules {
    pub rules: Vec<Rule>,
}

impl BoxxyRules {
    pub fn get_all_applicable_rules(&self, binary: &OsStr, fs: &FsDriver) -> Result<Vec<Rule>> {
        let mut applicable_rules = vec![];

        for rule in &self.rules {
            debug!("{}: checking if rule applies to binary", rule.name);
            if rule.currently_in_context(fs)? && rule.applies_to_binary(binary, fs)? {
                debug!("{}: rule applies to binary via only + context!", rule.name);
                applicable_rules.push(rule.clone());
            } else if rule.applies_to_binary(binary, fs)? {
                debug!(
                    "{}: rule applies to binary via only but NOT context!",
                    rule.name
                );
                applicable_rules.push(rule.clone());
            }
        }

        Ok(applicable_rules)
    }
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
    /// Environment variables that this rule applies if it matches. Any env
    /// vars listed here will be injected into the environment of the command
    /// that is being boxxed.
    #[serde(default = "empty_hashmap")]
    pub env: HashMap<String, String>,
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

        for rule_binary in &self.only {
            if self.test_program(program, &PathBuf::from(rule_binary), fs)? {
                debug!("{}: rule applies to binary!", self.name);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn test_program(&self, program: &OsStr, rule_binary: &Path, fs: &FsDriver) -> Result<bool> {
        debug!(
            "{}: testing program: program={program:?}, rule_binary={rule_binary:?}",
            self.name
        );

        // Compare program by file name, ex. ls == ls
        if let Some(file_name) = rule_binary.file_name() {
            debug!("{}: comparing file names: program={program:?}, rule binary file_name={file_name:?}", self.name);
            if program == file_name {
                return Ok(true);
            }
        }

        // Compare by given paths, ex. ls == /usr/bin/ls
        if let Some(path) = rule_binary.to_str() {
            debug!("{}: comparing binaries by given paths: program={program:?}, rule_binary={rule_binary:?}", self.name);
            if program == path {
                return Ok(true);
            }
        }

        // Fully expand rule path and program path, and compare. ex. /usr/bin/ls == /bin/ls
        let expanded_user_program = fs.fully_expand_path(&program.to_string_lossy().to_string())?;
        if let Ok(expanded_rule_binary) = rule_binary.canonicalize() {
            debug!("{}: comparing binaries by full expansion: expanded_user_program={expanded_user_program:?}, expanded_rule_binary={expanded_rule_binary:?}", self.name);
            if expanded_rule_binary == expanded_user_program {
                return Ok(true);
            }

            // Resolve rule path and program path as symlinks, and compare. ex. /bin/ls == /bin/ls
            let resolved_rule_binary = fs.maybe_resolve_symlink(&expanded_rule_binary)?;
            let resolved_user_program = fs.maybe_resolve_symlink(&expanded_user_program)?;
            debug!("{}: comparing binaries as resolved symlinks: resolved_user_program={resolved_user_program:?}, resolved_rule_binary={resolved_rule_binary:?}", self.name);
            if resolved_rule_binary == resolved_user_program {
                return Ok(true);
            }
        } else {
            // If we can't canonicalize the rule binary, try to resolve the
            // user program symlinks.
            let resolved_user_program = fs.maybe_resolve_symlink(&expanded_user_program)?;
            debug!("{}: comparing rule binary to user program as resolved symlinks: resolved_user_program={resolved_user_program:?}, rule_binary={rule_binary:?}", self.name);
            if let Some(file_name) = resolved_user_program.file_name() {
                if file_name == rule_binary {
                    debug!("{}: rule binary {rule_binary:?} matches user program file name for {resolved_user_program:?}", self.name);
                    return Ok(true);
                }
            } else if rule_binary == resolved_user_program {
                debug!("{}: rule binary {rule_binary:?} matches user program {resolved_user_program:?}", self.name);
                return Ok(true);
            }
        }

        // Resolve both program and rule_binary with `which` and compare. ex. /usr/bin/ls == /usr/bin/ls
        let which_rule_binary = match which::which(rule_binary) {
            Ok(which_rule_binary) => Some(which_rule_binary),
            Err(_) => None,
        };
        let which_user_program = match which::which(program) {
            Ok(which_user_program) => Some(which_user_program),
            Err(_) => None,
        };
        debug!("{}: comparing binaries with which(1): which_user_program={which_user_program:?}, which_rule_binary={which_rule_binary:?}", self.name);
        if which_rule_binary == which_user_program
            && (which_rule_binary.is_some() || which_user_program.is_some())
        {
            return Ok(true);
        }

        debug!("{}: rule didn't match anything, does not apply!", self.name);
        Ok(false)
    }
}

fn default_rule_mode() -> RuleMode {
    RuleMode::Directory
}

fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

fn empty_hashmap<K, V>() -> HashMap<K, V> {
    HashMap::new()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleMode {
    File,
    Directory,
}
