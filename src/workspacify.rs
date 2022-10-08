use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};
use structopt::StructOpt;
use toml_edit::{value, Array, Document, Formatted, InlineTable, Item, KeyMut, Table, Value};
use walkdir::WalkDir;

const FILES_HAVE_PARENTS: &str = "This is a file. Every file has a parent; qed";

#[derive(Debug, StructOpt)]
pub struct Workspacify {
    #[structopt(long)]
    path: Option<PathBuf>,
}

impl Workspacify {
    pub fn run(self) -> Result<()> {
        let workspace = self
            .path
            .map(Ok)
            .unwrap_or_else(|| current_dir().with_context(|| "Working directory is invalid."))?;
        ensure!(
            workspace.is_dir(),
            "Path '{}' is not a directory.",
            workspace.display()
        );

        // Create a mapping of package_name -> manifest
        let mut packages = HashMap::<String, PathBuf>::new();
        let mut duplicates = Vec::new();
        for manifest in manifest_iter(&workspace) {
            if let Some(name) = package_name(&manifest)? {
                if let Some(_) = packages.insert(name.clone(), manifest.clone()) {
                    duplicates.push(name);
                }
            }
        }
        if !duplicates.is_empty() {
            bail!("Duplicate crates detected: {:?}", duplicates);
        }

        // make sure all crates are recorded in the workspace manifest
        update_workspace_members(&workspace, &packages)
            .context("Failed to update member list in workspace manifest.")?;

        // transform every package manifest to point to the correct place
        // and use the correct version
        for (name, path) in packages.iter() {
            rewrite_manifest(path, &packages)
                .with_context(|| anyhow!("Failed to rewrite manifest for {}", name))?;
        }

        Ok(())
    }
}

fn manifest_iter(workspace: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(workspace)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_name() == "target" || e.file_name().to_string_lossy().starts_with("."))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy() == "Cargo.toml")
        .map(|dir| dir.into_path())
}

fn package_name(path: &Path) -> Result<Option<String>> {
    let ret = read_toml(path)?
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|p| p.get("name"))
        .and_then(|p| p.as_str())
        .map(Into::into);
    Ok(ret)
}

fn update_workspace_members(workspace: &Path, packages: &HashMap<String, PathBuf>) -> Result<()> {
    let manifest = {
        let mut workspace = workspace.to_path_buf();
        workspace.push("Cargo.toml");
        workspace
    };

    // turn packages into a sorted array of pathes
    let members: Array = {
        let mut members: Vec<_> = packages.iter().map(|(_, path)| path).collect();
        members.sort_unstable();
        let mut members: Array = members
            .iter()
            .map(|path| {
                let member = path
                    .parent()
                    .expect(FILES_HAVE_PARENTS)
                    .strip_prefix(workspace)
                    .expect(FILES_HAVE_PARENTS)
                    .to_string_lossy()
                    .into_owned();
                let mut formatted = Formatted::new(member);
                formatted.decor_mut().set_prefix("\n\t");
                Value::String(formatted)
            })
            .collect();
        members.set_trailing("\n");
        members.set_trailing_comma(true);
        members
    };

    // create the workspace manifest if it does't exist
    let mut content = String::new();
    OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&manifest)
        .with_context(|| anyhow!("Failed to to open {}", manifest.display()))?
        .read_to_string(&mut content)
        .with_context(|| "Failed to read workspace manifest")?;

    let mut toml = Document::from_str(&content)?;
    toml.entry("workspace")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("`workspace` is not a table"))?
        .insert("members", value(members));

    Ok(fs::write(&manifest, toml.to_string())?)
}

fn rewrite_manifest(path: &Path, packages: &HashMap<String, PathBuf>) -> Result<()> {
    let mut toml = read_toml(path)?;

    toml.iter_mut()
        .filter(|(k, _)| k.contains("dependencies"))
        .filter_map(|(_, v)| v.as_table_mut())
        .flat_map(|deps| deps.iter_mut())
        .filter_map(|dep| dep.1.as_inline_table_mut().map(|v| (dep.0, v)))
        .try_for_each(|dep| handle_dep((dep.0, dep.1, path), packages))?;

    Ok(fs::write(&path, toml.to_string())?)
}

fn handle_dep(
    dep: (KeyMut, &mut InlineTable, &Path),
    packages: &HashMap<String, PathBuf>,
) -> Result<()> {
    let name = dep
        .1
        .get("package")
        .and_then(|p| p.as_str())
        .unwrap_or(dep.0.get());

    // dependency exists within this workspace
    let (dependee, dependency) = if let Some(path) = packages.get(name) {
        let dependee = path.parent().expect(FILES_HAVE_PARENTS);
        let dependency = dep.2.parent().expect(FILES_HAVE_PARENTS);
        (dependee, dependency)
    } else {
        return Ok(());
    };

    // path in manifests are relative
    let relpath = pathdiff::diff_paths(dependee, dependency).ok_or_else(|| {
        anyhow!(
            "Cannot make {} relative to {}",
            dependee.display(),
            dependency.display()
        )
    })?;
    dep.1.remove("git");
    dep.1.remove("branch");
    dep.1.remove("version");
    dep.1
        .insert("path", Value::from(relpath.to_string_lossy().as_ref()));
    dep.1
        .sort_values_by(|k0, _, k1, _| dep_key_order(k0).cmp(&dep_key_order(k1)));

    Ok(())
}

fn read_toml(path: &Path) -> Result<Document> {
    let content = fs::read_to_string(path)?;
    Ok(Document::from_str(&content)?)
}

fn dep_key_order(dep_key: &str) -> u32 {
    match dep_key {
        "package" => 0,

        "git" => 10,
        "path" => 10,

        "version" => 30,
        "branch" => 30,
        "tag" => 30,

        "default-features" => 40,

        "features" => 50,

        "optional" => 60,

        _ => u32::MAX,
    }
}
