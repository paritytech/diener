use git_url_parse::GitUrl;
use std::{env::current_dir, fs, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use toml_edit::{decorated, Document, InlineTable};
use walkdir::WalkDir;

/// Which dependencies should be rewritten?
#[derive(Debug, Clone)]
enum Rewrite {
    All,
    Substrate(Option<String>),
    Polkadot(Option<String>),
    Cumulus(Option<String>),
    Beefy(Option<String>),
}

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

    /// Only alter Substrate dependencies.
    #[structopt(long, short = "s")]
    substrate: bool,

    /// Only alter Polkadot dependencies.
    #[structopt(long, short = "p")]
    polkadot: bool,

    /// Only alter Cumulus dependencies.
    #[structopt(long, short = "c")]
    cumulus: bool,

    /// Only alter BEEFY dependencies.
    #[structopt(long, short = "b")]
    beefy: bool,

    /// Alter polkadot, substrate + beefy dependencies
    #[structopt(long, short = "a")]
    all: bool,

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
    /// Convert the options into the parts `Rewrite`, `Version`, `Option<PathBuf>`.
    fn into_parts(self) -> Result<(Rewrite, Version, Option<PathBuf>), String> {
        let version = if let Some(branch) = self.branch {
            Version::Branch(branch)
        } else if let Some(rev) = self.rev {
            Version::Rev(rev)
        } else if let Some(tag) = self.tag {
            Version::Tag(tag)
        } else {
            return Err("You need to pass `--branch`, `--tag` or `--rev`".into());
        };

        let rewrite = if self.all {
            if self.git.is_some() {
                return Err("You need to pass `--substrate`, `--polkadot`, `--cumulus` or `--beefy` for `--git`.".into());
            } else {
                Rewrite::All
            }
        } else if self.substrate {
            Rewrite::Substrate(self.git)
        } else if self.beefy {
            Rewrite::Beefy(self.git)
        } else if self.polkadot {
            Rewrite::Polkadot(self.git)
        } else if self.cumulus {
            Rewrite::Cumulus(self.git)
        } else {
            return Err("You must specify one of `--substrate`, `--polkadot`, `--cumulus`, `--beefy` or `--all`.".into())
        };

        Ok((rewrite, version, self.path))
    }

    /// Run this subcommand.
    pub fn run(self) -> Result<(), String> {
        let (rewrite, version, path) = self.into_parts()?;

        let path = path.map(Ok).unwrap_or_else(|| {
            current_dir().map_err(|e| format!("Working directory is invalid: {:?}", e))
        })?;

        WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.file_name().to_string_lossy().ends_with("Cargo.toml")
            })
            .try_for_each(|toml| handle_toml_file(toml.into_path(), &rewrite, &version))
    }
}

/// Handle a given dependency.
///
/// This directly modifies the given `dep` in the requested way.
fn handle_dependency(dep: &mut InlineTable, rewrite: &Rewrite, version: &Version) {
    let git = if let Some(git) = dep
        .get("git")
        .and_then(|v| v.as_str())
        .and_then(|d| GitUrl::parse(&d).ok())
    {
        git
    } else {
        return;
    };

    let new_git = match rewrite {
        Rewrite::All => &None,
        Rewrite::Substrate(new_git) if git.name == "substrate" => new_git,
        Rewrite::Polkadot(new_git) if git.name == "polkadot" => new_git,
        Rewrite::Cumulus(new_git) if git.name == "cumulus" => new_git,
        Rewrite::Beefy(new_git) if git.name == "grandpa-bridge-gadget" => new_git,
        _ => return,
    };

    if let Some(new_git) = new_git {
        *dep.get_or_insert("git", "") = decorated(new_git.as_str().into(), " ", "");
    }

    dep.remove("tag");
    dep.remove("branch");
    dep.remove("rev");

    match version {
        Version::Tag(tag) => {
            *dep.get_or_insert(" tag", "") = decorated(tag.as_str().into(), " ", " ");
        }
        Version::Branch(branch) => {
            *dep.get_or_insert(" branch", "") = decorated(branch.as_str().into(), " ", " ");
        }
        Version::Rev(rev) => {
            *dep.get_or_insert(" rev", "") = decorated(rev.as_str().into(), " ", " ");
        }
    }
}

/// Handle a given `Cargo.toml`.
///
/// This means scanning all dependencies and rewrite the requested onces.
fn handle_toml_file(path: PathBuf, rewrite: &Rewrite, version: &Version) -> Result<(), String> {
    println!("Processing: {}", path.display());

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to open `{}`: {:?}", path.display(), e))?;
    let mut toml_doc = Document::from_str(&content)
        .map_err(|e| format!("Failed to parse as toml doc: {:?}", e))?;

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
                    handle_dependency(table, rewrite, version);
                })
        });

    fs::write(&path, toml_doc.to_string_in_original_order())
        .map_err(|e| format!("Failed to write to `{}`: {:?}", path.display(), e))
}
