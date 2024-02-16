/*!

diener - dependency diener is a tool for easily changing [Polkadot SDK](https://github.com/paritytech/polkadot) dependency versions

[![](https://docs.rs/diener/badge.svg)](https://docs.rs/diener/) [![](https://img.shields.io/crates/v/diener.svg)](https://crates.io/crates/diener) [![](https://img.shields.io/crates/d/diener.png)](https://crates.io/crates/diener)

* [Usage](#usage)
* [License](#license)

## Usage

### Update

The `update` subcommand changes all `Cargo.toml` files in a given folder to use
a specific branch/path/commit/tag.

Change all Polkadot SDK dependencies in a folder to a different branch:

```rust
diener update --branch diener-branch
```

Diener also supports `tag` and `rev` as arguments.

### Patch

The `patch` subcommand adds a patch section for each crate in a given cargo workspace
to the workspace `Cargo.toml` file in some other cargo workspace.

Patch all git dependencies to be build from a given path:

```rust
diener patch --crates-to-patch ../path/to/polkadot-sdk/checkout
```

This subcommand can be compared to `.cargo/config` without using a deprecated
feature of Cargo ;)

## License

Licensed under either of

 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)

 * [MIT license](http://opensource.org/licenses/MIT)

at your option.
*/

use env_logger::Env;
use structopt::{
    clap::{crate_name, crate_version},
    StructOpt,
};

mod patch;
mod update;
mod workspacify;

/// diener is a tool for easily finding and changing Polkadot SDK dependency versions.
/// diener will not modified the cargo.lock file but update specific dependencies in the Cargo.toml files or the project.
#[derive(Debug, StructOpt)]
enum SubCommands {
    /// Update all `Cargo.toml` files at a given path to some specific path/branch/commit.
    Update(update::Update),
    /// Patch all crates from a given cargo workspace in another given cargo workspace.
    ///
    /// This will get all crates from a given cargo workspace and add a patch
    /// section for each of these crates to the workspace `Cargo.toml` of a
    /// given cargo workspace. Essentially this is the same as using
    /// `.cargo/config`, but using a non-deprecated way.
    Patch(patch::Patch),
    /// Creates a workspace from the supplied directory tree.
    ///
    /// This can be ran on existing workspaces to make sure everything is properly setup.
    ///
    /// - Every dependency residing in the tree will be rewritten into a `path` dependency.
    /// - The top level `Cargo.toml` `workspace.members` array will be filled with all crates.
    ///     - It will also be sorted alphabetically
    /// - The path dependency entries will be sorted into a canonical order.
    Workspacify(workspacify::Workspacify),
}

/// Cli options of Diener
#[derive(Debug, StructOpt)]
#[structopt(
    about = "Diener - dependency diener for replacing Polkadot SDK versions in `Cargo.toml` files"
)]
struct Options {
    #[structopt(subcommand)]
    subcommand: SubCommands,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    log::info!("Running {} v{}", crate_name!(), crate_version!());

    match Options::from_args().subcommand {
        SubCommands::Update(update) => update.run(),
        SubCommands::Patch(patch) => patch.run(),
        SubCommands::Workspacify(workspacify) => workspacify.run(),
    }
}
