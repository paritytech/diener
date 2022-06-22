use std::env::current_dir;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use toml_edit::{Document, Item, Value};
use walkdir::WalkDir;

/// `check-features` subcommand options.
#[derive(Debug, StructOpt)]
pub struct CheckFeatures {
    /// The path where Diener should search for `Cargo.toml` files.
    #[structopt(long)]
    path: Option<PathBuf>,
}

impl CheckFeatures {
    /// Run this subcommand.
    pub fn run(self) -> Result<(), String> {
        let path = self
            .path
            .unwrap_or(current_dir().map_err(|e| format!("Working directory is invalid: {e}"))?);
        if !path.is_dir() {
            return Err(format!("Path '{}' is not a directory.", path.display()));
        }

        WalkDir::new(&path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.file_name() == "Cargo.toml")
            .for_each(|toml| {
                if let Err(e) = check_toml(toml.into_path()) {
                    log::debug!("Failed to check {}: {}", path.display(), e);
                }
            });
        Ok(())
    }
}

/// Check the given `Cargo.toml`.
///
/// Prints a list of dependencies that have `default-features = false` and are not part of the
/// `std` feature.
fn check_toml<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();
    let toml = parse_toml(path)?;

    let non_default_features_deps = get_non_default_features_deps(&toml)?;
    let std_crates = get_std_crates(&toml)?;
    for dep in non_default_features_deps {
        if !std_crates.contains(&dep) {
            println!(
                "{}: {} has `default-features = false` but is not present in feature `std`",
                path.display(),
                dep
            );
        }
    }
    Ok(())
}

/// Return a list of `[dependencies]` from the provided toml where `default-features = false`.
fn get_non_default_features_deps(toml: &Document) -> Result<Vec<String>, String> {
    let deps = match toml
        .get("dependencies")
        .ok_or(format!("No 'dependency' section found in `Cargo.toml`"))?
    {
        Item::Table(table) => table.get_values(),
        _ => Err(format!(
            "Failed to parse 'dependency' section in `Cargo.toml` as table"
        ))?,
    };

    let deps = deps
        .iter()
        .filter_map(|(keys, value)| {
            if let Value::InlineTable(dep_spec) = value {
                if let Some((_key, value)) = dep_spec.get_key_value("default-features") {
                    let default_features = value.as_bool()?;
                    if !default_features {
                        return Some((keys[0] as &str).to_string());
                    }
                }
            }
            None
        })
        .collect::<Vec<String>>();
    Ok(deps)
}

/// Return a list of crates included if the `std` feature is enabled.
fn get_std_crates(toml: &Document) -> Result<Vec<String>, String> {
    let (_key, values) = match toml
        .get("features")
        .ok_or(format!("No 'features' section found in `Cargo.toml`"))?
    {
        Item::Table(table) => table
            .get_key_value("std")
            .ok_or(format!("No 'std' feature in `Cargo.toml`"))?,
        _ => Err(format!(
            "Failed to parse 'features' section in `Cargo.toml` as table"
        ))?,
    };
    let values = values
        .as_array()
        .ok_or(format!(
            "Failed to parse 'std' feature in `Cargo.toml` as array"
        ))?
        .iter()
        .filter_map(|val| val.as_str())
        .filter_map(|val| val.split('/').nth(0))
        .map(|val| val.to_string())
        .collect::<Vec<String>>();
    Ok(values)
}

/// Parse the given TOML to a `toml_edit::Document`
fn parse_toml<P: AsRef<Path>>(path: P) -> Result<Document, String> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let toml = contents
        .parse::<Document>()
        .map_err(|e| format!("Failed to parse {} as toml: {}", path.display(), e))?;
    Ok(toml)
}
