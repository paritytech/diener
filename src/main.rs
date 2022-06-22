/*!

diener - dependency diener is a tool for easily changing [Substrate](https://github.com/paritytech/substrate), [Polkadot](https://github.com/paritytech/polkadot) and [Cumulus](https://github.com/paritytech/cumulus) dependency versions

[![](https://docs.rs/diener/badge.svg)](https://docs.rs/diener/) [![](https://img.shields.io/crates/v/diener.svg)](https://crates.io/crates/diener) [![](https://img.shields.io/crates/d/diener.png)](https://crates.io/crates/diener)

* [Usage](#usage)
* [License](#license)

## Usage

### Update

The `update` subcommand changes all `Cargo.toml` files in a given folder to use
a specific branch/path/commit/tag.

Change all Substrate dependencies in a folder to a different branch:

```
diener update --substrate --branch diener-branch
```

Or you want to change Polkadot, Substrate and Cumulus dependencies to the same branch:

```
diener update --branch diener-branch-2
```

Diener also supports `tag` and `rev` as arguments.

If a depdendency is belongs to Substrate, Polkadot or Cumulus is currently done by looking at the git url.
It also only works for repos called `substrate`, `polkadot` or `cumulus`.

### Patch

The `patch` subcommand adds a patch section for each crate in a given cargo workspace
to the workspace `Cargo.toml` file in some other cargo workspace.

Patch all Substrate git dependencies to be build from a given path:

```
diener patch --crates-to-patch ../path/to/substrate/checkout --substrate
```

This subcommand can be compared to `.cargo/config` without using a deprecated
feature of Cargo ;)

### check-features

The `check-features` subcommand checks all `Cargo.toml` files in a given folder for
dependencies that have `default-features = false` but are not part of the `std` feature.

```
diener check-features
```

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

mod check_features;
mod patch;
mod update;

/// diener is a tool for easily finding and changing Substrate or Polkadot dependency versions.
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
    /// Check all `Cargo.toml` files at a given path for dependencies that have
    /// `default-features = false` but are not part of the `std` feature.
    CheckFeatures(check_features::CheckFeatures),
}

/// Cli options of Diener
#[derive(Debug, StructOpt)]
#[structopt(
    about = "Diener - dependency diener for replacing substrate, polkadot, cumulus or beefy versions in `Cargo.toml` files"
)]
struct Options {
    #[structopt(subcommand)]
    subcommand: SubCommands,
}

fn main() -> Result<(), String> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    log::info!("Running {} v{}", crate_name!(), crate_version!());

    match Options::from_args().subcommand {
        SubCommands::Update(update) => update.run(),
        SubCommands::Patch(patch) => patch.run(),
        SubCommands::CheckFeatures(check) => check.run(),
    }
}
