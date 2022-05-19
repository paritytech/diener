use std::{
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use structopt::StructOpt;
use toml_edit::{decorated, Document, Item, Value};
use walkdir::WalkDir;

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
    ) -> Result<Self, String> {
        if let Some(repository) = point_to_git {
            if let Some(branch) = point_to_git_branch {
                Ok(Self::GitBranch { repository, branch })
            } else if let Some(commit) = point_to_git_commit {
                Ok(Self::GitCommit { repository, commit })
            } else {
                Err("`--point-to-git-branch` or `--point-to-git-commit` are required when `--point-to-git` is passed!"
					.into())
            }
        } else {
            Ok(Self::Path)
        }
    }
}

impl PatchTarget {
    /// Returns the patch target in a toml compatible format.
    fn as_string(&self) -> String {
        match self {
            Self::Crates => "crates-io".into(),
            Self::Git(url) => format!("\"{}\"", url),
            Self::Custom(custom) => format!("\"{}\"", custom),
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
    ///
    /// The target is `[patch.TARGET]` in the final `Cargo.toml`.
    #[structopt(
        long,
        conflicts_with_all = &[ "crates", "substrate", "cumulus", "polkadot", "beefy" ]
    )]
    target: Option<String>,

    /// Use the official Substrate repo as patch target.
    #[structopt(
        long,
        short = "s",
        conflicts_with_all = &[ "target", "polkadot", "cumulus", "crates", "beefy" ]
    )]
    substrate: bool,

    /// Use the official Polkadot repo as patch target.
    #[structopt(
        long,
        short = "p",
        conflicts_with_all = &[ "target", "substrate", "cumulus", "crates", "beefy" ]
    )]
    polkadot: bool,

    /// Use the official Cumulus repo as patch target.
    #[structopt(
        long,
        short = "c",
        conflicts_with_all = &[ "target", "substrate", "polkadot", "crates", "beefy" ]
    )]
    cumulus: bool,

    /// Use the official BEEFY repo as patch target.
    #[structopt(
        long,
        short = "b",
        conflicts_with_all = &[ "target", "substrate", "cumulus", "crates", "polkadot" ]
    )]
    beefy: bool,

    /// Use `crates.io` as patch target.
    #[structopt(
        long,
        conflicts_with_all = &[ "target", "substrate", "polkadot", "cumulus", "beefy" ]
    )]
    crates: bool,
}

impl Patch {
    /// Run this subcommand.
    pub fn run(self) -> Result<(), String> {
        let patch_target = self.patch_target()?;

        let path = self
            .path
            .map(|p| {
                if !p.exists() {
                    Err(format!("Given --path=`{}` does not exist!", p.display()))
                } else {
                    Ok(p)
                }
            })
            .unwrap_or_else(|| {
                current_dir().map_err(|e| format!("Working directory is invalid: {:?}", e))
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
            workspace_packages(&self.crates_to_patch),
            point_to,
        )
    }

    fn patch_target(&self) -> Result<PatchTarget, String> {
        if let Some(ref custom) = self.target {
            Ok(PatchTarget::Custom(custom.clone()))
        } else if self.substrate {
            Ok(PatchTarget::Git(
                "https://github.com/paritytech/substrate".into(),
            ))
        } else if self.polkadot {
            Ok(PatchTarget::Git(
                "https://github.com/paritytech/polkadot".into(),
            ))
        } else if self.cumulus {
            Ok(PatchTarget::Git(
                "https://github.com/paritytech/cumulus".into(),
            ))
        } else if self.beefy {
            Ok(PatchTarget::Git(
                "https://github.com/paritytech/parity-bridges-gadget".into(),
            ))
        } else if self.crates {
            Ok(PatchTarget::Crates)
        } else {
            Err("You need to pass `--target`, `--substrate`, `--polkadot`, `--cumulus`, `--beefy` or `--crates`!"
				.into())
        }
    }
}

fn workspace_root_package(path: &Path) -> Result<PathBuf, String> {
    if path.ends_with("Cargo.toml") {
        return Ok(path.into());
    }

    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(path)
        .exec()
        .map_err(|e| {
            format!(
                "Failed to get cargo metadata for workspace `{}`: {:?}",
                path.display(),
                e
            )
        })?;

    Ok(metadata.workspace_root.join("Cargo.toml").into())
}

/// Returns all package names of the given `workspace`.
struct PackageInfo {
    cargo_toml_dir: PathBuf,
    name: String,
}
fn workspace_packages(workspace: &Path) -> impl Iterator<Item = PackageInfo> {
    WalkDir::new(workspace)
        .follow_links(true)
        .into_iter()
        .filter_map(|file| {
            let file = file.ok()?;
            if file.file_type().is_file() && file.file_name().to_string_lossy() == "Cargo.toml" {
                let cargo_toml_dir = {
                    let mut path = file.path().to_path_buf();
                    path.pop(); // Remove the "/Cargo.toml" at the end
                    path
                };

                // Skip the file if it's within a hidden directory
                for path_segment in cargo_toml_dir.iter() {
                    if let Some(path_segment) = path_segment.to_str() {
                      if path_segment.starts_with('.') {
                          log::debug!(
                              "Skipping file {:?} because its segment {:?} indicates it's within a hidden directory",
                              &file.path(),
                              path_segment,
                          );
                          return None;
                      }
                    } else {
                        log::error!(
                            "Failed to parse path segment {:?} of file {:?}",
                            path_segment,
                            &file.path(),
                        );
                        return None;
                    }
                }

                let content = fs::read_to_string(&file.path())
                    .map_err(|err| {
                        log::error!("Failed to read file {:?} due to {:?}", &file.path(), err)
                    })
                    .ok()?;

                let toml_doc = Document::from_str(&content)
                    .map_err(|err| {
                        log::error!(
                            "Failed to parse {:?} as TOML due to {:?}",
                            &file.path(),
                            err
                        )
                    })
                    .ok()?;

                if let Some(pkg_name) = toml_doc.as_table()["package"]["name"].as_str() {
                    Some(PackageInfo {
                        cargo_toml_dir,
                        name: pkg_name.into(),
                    })
                } else {
                    log::error!(
                        "Failed to get the package name of {:?} as a string",
                        &file.path()
                    );
                    None
                }
            } else {
                None
            }
        })
}

fn add_patches_for_packages(
    cargo_toml: &Path,
    patch_target: &PatchTarget,
    mut packages: impl Iterator<Item = PackageInfo>,
    point_to: PointTo,
) -> Result<(), String> {
    let content = fs::read_to_string(cargo_toml)
        .map_err(|e| format!("Could not read `{}`: {:?}", cargo_toml.display(), e))?;
    let mut doc = Document::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse `{}` as `Cargo.toml`: {:?}",
            cargo_toml.display(),
            e
        )
    })?;

    let patch_table = doc
        .as_table_mut()
        .entry("patch")
        .or_insert(Item::Table(Default::default()))
        .as_table_mut()
        .ok_or("Patch table isn't a toml table!")?;

    patch_table.set_implicit(true);

    let patch_target_table = patch_table
        .entry(&patch_target.as_string())
        .or_insert(Item::Table(Default::default()))
        .as_table_mut()
        .ok_or("Patch target table isn't a toml table!")?;

    packages.try_for_each(|pkg| {
        log::info!("Adding patch for `{}`.", pkg.name);

        let patch = patch_target_table
            .entry(&pkg.name)
            .or_insert(Item::Value(Value::InlineTable(Default::default())))
            .as_inline_table_mut()
            .ok_or(format!(
                "Patch entry for `{}` isn't an inline table!",
                pkg.name
            ))?;

        match &point_to {
            PointTo::Path => {
                *patch.get_or_insert("path", "") =
                    decorated(pkg.cargo_toml_dir.display().to_string().into(), " ", " ");
            }
            PointTo::GitBranch { repository, branch } => {
                *patch.get_or_insert("git", "") = decorated(repository.clone().into(), " ", " ");
                *patch.get_or_insert("branch", "") = decorated(branch.clone().into(), " ", " ");
            }
            PointTo::GitCommit { repository, commit } => {
                *patch.get_or_insert("git", "") = decorated(repository.clone().into(), " ", " ");
                *patch.get_or_insert("rev", "") = decorated(commit.clone().into(), " ", " ");
            }
        }
        Ok::<_, String>(())
    })?;

    fs::write(&cargo_toml, doc.to_string_in_original_order())
        .map_err(|e| format!("Failed to write to `{}`: {:?}", cargo_toml.display(), e))
}
