use serde::{Deserialize, Serialize};

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

fn default_rule_mode() -> RuleMode {
    RuleMode::Directory
}

fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RuleMode {
    File,
    Directory,
}
