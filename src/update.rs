use anyhow::{bail, ensure, Context, Result};
use git_url_parse::GitUrl;
use std::{env::current_dir, fs, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use toml_edit::{Document, InlineTable, Value};
use walkdir::{DirEntry, WalkDir};

/// The version the dependencies should be switched to.
#[derive(Debug, Clone)]
enum Version {
    Tag(String),
    Branch(String),
    Rev(String),
}

/// `update` subcommand options.
#[derive(Debug, StructOpt)]
pub struct Update {
    /// The path where Diener should search for `Cargo.toml` files.
    #[structopt(long)]
    path: Option<PathBuf>,

    /// The `branch` that the dependencies should use.
    #[structopt(long, conflicts_with_all = &[ "rev", "tag" ])]
    branch: Option<String>,

    /// The `rev` that the dependencies should use.
    #[structopt(long, conflicts_with_all = &[ "branch", "tag" ])]
    rev: Option<String>,

    /// The `tag` that the dependencies should use.
    #[structopt(long, conflicts_with_all = &[ "rev", "branch" ])]
    tag: Option<String>,

    /// Rewrite the `git` url to the give one.
    #[structopt(long)]
    git: Option<String>,
}

impl Update {
    /// Convert the options into the parts `Option<String>`, `Version`, `Option<PathBuf>`.
    fn into_parts(self) -> Result<(Option<String>, Version, Option<PathBuf>)> {
        let version = if let Some(branch) = self.branch {
            Version::Branch(branch)
        } else if let Some(rev) = self.rev {
            Version::Rev(rev)
        } else if let Some(tag) = self.tag {
            Version::Tag(tag)
        } else {
            bail!("You need to pass `--branch`, `--tag` or `--rev`");
        };

        Ok((self.git, version, self.path))
    }

    /// Run this subcommand.
    pub fn run(self) -> Result<()> {
        let (git, version, path) = self.into_parts()?;

        let path = path
            .map(Ok)
            .unwrap_or_else(|| current_dir().with_context(|| "Working directory is invalid."))?;
        ensure!(
            path.is_dir(),
            "Path '{}' is not a directory.",
            path.display()
        );

        let is_hidden = |entry: &DirEntry| {
            entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        };

        WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.file_name().to_string_lossy().ends_with("Cargo.toml")
            })
            .try_for_each(|toml| handle_toml_file(toml.into_path(), &git, &version))
    }
}

/// Handle a given dependency.
///
/// This directly modifies the given `dep` in the requested way.
fn handle_dependency(name: &str, dep: &mut InlineTable, git: &Option<String>, version: &Version) {
    if !dep
        .get("git")
        .and_then(|v| v.as_str())
        .and_then(|d| GitUrl::parse(d).ok())
        .is_some_and(|git| git.name == "polkadot-sdk")
    {
        return;
    }

    if let Some(new_git) = git {
        *dep.get_or_insert("git", "") = Value::from(new_git.as_str()).decorated(" ", "");
    }

    dep.remove("tag");
    dep.remove("branch");
    dep.remove("rev");

    // Workspace dependencies cannot use .tag, .branch or .rev
    // Turn the workspace dependency into a normal dependency before patching it
    dep.remove("workspace");

    match version {
        Version::Tag(tag) => {
            *dep.get_or_insert("tag", "") = Value::from(tag.as_str()).decorated(" ", " ");
        }
        Version::Branch(branch) => {
            *dep.get_or_insert("branch", "") = Value::from(branch.as_str()).decorated(" ", " ");
        }
        Version::Rev(rev) => {
            *dep.get_or_insert("rev", "") = Value::from(rev.as_str()).decorated(" ", " ");
        }
    }
    log::debug!("  updated: {:?} <= {}", version, name);
}

/// Handle a given `Cargo.toml`.
///
/// This means scanning all dependencies and rewrite the requested onces.
fn handle_toml_file(path: PathBuf, git: &Option<String>, version: &Version) -> Result<()> {
    log::info!("Processing: {}", path.display());

    let mut toml_doc = Document::from_str(&fs::read_to_string(&path)?)?;

    // Iterate over all tables in the document
    toml_doc
        .clone()
        .iter()
        // filter out everything that is not a dependency table
        .filter(|(k, _)| k.contains("dependencies"))
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .for_each(|(k, t)| {
            t.iter()
                // Filter everything that is not an inline table (`{ foo = bar }`)
                .filter_map(|v| v.1.as_inline_table().map(|_| v.0))
                .for_each(|dn| {
                    // Get the actual inline table from the document that we modify
                    let table = toml_doc[k][dn]
                        .as_inline_table_mut()
                        .expect("We filter by `is_inline_table`; qed");
                    handle_dependency(dn, table, git, version);
                })
        });

    fs::write(&path, toml_doc.to_string())?;
    Ok(())
}
