use anyhow::{anyhow, bail, Context, Error, Result};
use std::{
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use structopt::StructOpt;
use toml_edit::{Document, Item, Value};

enum PatchTarget {
    Crates,
    Git(String),
    Custom(String),
}

/// Where should the patch point to?
enum PointTo {
    /// Point to the crate path.
    Path,
    /// Point to the git branch.
    GitBranch { repository: String, branch: String },
    /// Point to the git commit.
    GitCommit { repository: String, commit: String },
}

impl PointTo {
    fn from_cli(
        point_to_git: Option<String>,
        point_to_git_branch: Option<String>,
        point_to_git_commit: Option<String>,
    ) -> Result<Self> {
        if let Some(repository) = point_to_git {
            if let Some(branch) = point_to_git_branch {
                Ok(Self::GitBranch { repository, branch })
            } else if let Some(commit) = point_to_git_commit {
                Ok(Self::GitCommit { repository, commit })
            } else {
                bail!("`--point-to-git-branch` or `--point-to-git-commit` are required when `--point-to-git` is passed!");
            }
        } else {
            Ok(Self::Path)
        }
    }
}

impl PatchTarget {
    /// Returns the patch target in a toml compatible format.
    fn as_str(&self) -> &str {
        match self {
            Self::Crates => "crates-io",
            Self::Git(url) => url,
            Self::Custom(custom) => custom,
        }
    }
}

/// `patch` subcommand options.
#[derive(Debug, StructOpt)]
pub struct Patch {
    /// The path to the project where the patch section should be added.
    ///
    /// If not given, the current directory will be taken.
    ///
    /// If this points to a `Cargo.toml` file, this file will be taken as the
    /// cargo workspace `Cargo.toml` file to add the patches.
    ///
    /// The patches will be added to the cargo workspace `Cargo.toml` file.
    #[structopt(long)]
    path: Option<PathBuf>,

    /// The workspace that should be scanned and added to the patch section.
    ///
    /// This will execute `cargo metadata` in the given workspace and add
    /// all packages of this workspace to the patch section.
    #[structopt(long)]
    crates_to_patch: PathBuf,

    /// Instead of using the path to the crates, use the given git repository.
    ///
    /// This requires that either `--point-to-git-commit` or
    /// `--point-to-git-branch` is given as well.
    #[structopt(long)]
    point_to_git: Option<String>,

    /// Specify the git branch that should be used with the repository given
    /// to `--point-to-git`.
    #[structopt(
        long,
        conflicts_with_all = &[ "point-to-git-commit" ],
        requires_all = &[ "point-to-git" ],
    )]
    point_to_git_branch: Option<String>,

    /// Specify the git commit that should be used with the repository given
    /// to `--point-to-git`.
    #[structopt(
        long,
        conflicts_with_all = &[ "point-to-git-branch" ],
        requires_all = &[ "point-to-git" ],
    )]
    point_to_git_commit: Option<String>,

    /// The patch target that should be used.
    /// The default is the official `polkadot-sdk` repository.
    ///
    /// The target is `[patch.TARGET]` in the final `Cargo.toml`.
    #[structopt(
        long,
        conflicts_with_all = &[ "crates" ]
    )]
    target: Option<String>,

    /// Use `crates.io` as patch target instead.
    #[structopt(
        long,
        conflicts_with_all = &[ "target" ]
    )]
    crates: bool,
}

impl Patch {
    /// Run this subcommand.
    pub fn run(self) -> Result<()> {
        let patch_target = self.patch_target();
        let path = self
            .path
            .map(|p| {
                if !p.exists() {
                    bail!("Given --path=`{}` does not exist!", p.display());
                } else {
                    Ok(p)
                }
            })
            .unwrap_or_else(|| {
                current_dir().with_context(|| anyhow!("Working directory is invalid."))
            })?;

        // Get the path to the `Cargo.toml` where we need to add the patches
        let cargo_toml_to_patch = workspace_root_package(&path)?;

        let point_to = PointTo::from_cli(
            self.point_to_git,
            self.point_to_git_branch,
            self.point_to_git_commit,
        )?;

        add_patches_for_packages(
            &cargo_toml_to_patch,
            &patch_target,
            workspace_packages(&self.crates_to_patch)?,
            point_to,
        )
    }

    fn patch_target(&self) -> PatchTarget {
        if let Some(ref custom) = self.target {
            PatchTarget::Custom(custom.clone())
        } else if self.crates {
            PatchTarget::Crates
        } else {
            PatchTarget::Git("https://github.com/paritytech/polkadot-sdk".into())
        }
    }
}

fn workspace_root_package(path: &Path) -> Result<PathBuf> {
    if path.ends_with("Cargo.toml") {
        return Ok(path.into());
    }

    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(path)
        .exec()
        .with_context(|| "Failed to get cargo metadata for workspace")?;

    Ok(metadata.workspace_root.join("Cargo.toml").into())
}

/// Returns all package names of the given `workspace`.
fn workspace_packages(workspace: &Path) -> Result<impl Iterator<Item = cargo_metadata::Package>> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(workspace)
        .exec()
        .with_context(|| "Failed to get cargo metadata for workspace.")?;

    Ok(metadata
        .workspace_members
        .clone()
        .into_iter()
        .map(move |p| metadata[&p].clone()))
}

fn add_patches_for_packages(
    cargo_toml: &Path,
    patch_target: &PatchTarget,
    mut packages: impl Iterator<Item = cargo_metadata::Package>,
    point_to: PointTo,
) -> Result<()> {
    let content = fs::read_to_string(cargo_toml)
        .with_context(|| anyhow!("Failed to read manifest at {}", cargo_toml.display()))?;
    let mut doc = Document::from_str(&content).context("Failed to parse Cargo.toml")?;

    let patch_table = doc
        .as_table_mut()
        .entry("patch")
        .or_insert(Item::Table(Default::default()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("Patch table isn't a toml table!"))?;

    patch_table.set_implicit(true);

    let patch_target_table = patch_table
        .entry(patch_target.as_str())
        .or_insert(Item::Table(Default::default()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("Patch target table isn't a toml table!"))?;

    packages.try_for_each(|mut p| {
        log::info!("Adding patch for `{}`.", p.name);

        let patch = patch_target_table
            .entry(&p.name)
            .or_insert(Item::Value(Value::InlineTable(Default::default())))
            .as_inline_table_mut()
            .ok_or_else(|| anyhow!("Patch entry for `{}` isn't an inline table!", p.name))?;

        if p.manifest_path.ends_with("Cargo.toml") {
            p.manifest_path.pop();
        }

        let path: PathBuf = p.manifest_path.into();

        match &point_to {
            PointTo::Path => {
                *patch.get_or_insert("path", "") =
                    Value::from(path.display().to_string()).decorated(" ", " ");
            }
            PointTo::GitBranch { repository, branch } => {
                *patch.get_or_insert("git", "") =
                    Value::from(repository.clone()).decorated(" ", " ");
                *patch.get_or_insert("branch", "") =
                    Value::from(branch.clone()).decorated(" ", " ");
            }
            PointTo::GitCommit { repository, commit } => {
                *patch.get_or_insert("git", "") =
                    Value::from(repository.clone()).decorated(" ", " ");
                *patch.get_or_insert("rev", "") = Value::from(commit.clone()).decorated(" ", " ");
            }
        }
        Ok::<_, Error>(())
    })?;

    fs::write(cargo_toml, doc.to_string())
        .with_context(|| anyhow!("Failed to write manifest to {}", cargo_toml.display()))
}
