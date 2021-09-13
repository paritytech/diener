# diener - dependency diener is a tool for easily changing [Substrate](https://github.com/paritytech/substrate) or [Polkadot](https://github.com/paritytech/polkadot) dependency versions

[![](https://docs.rs/diener/badge.svg)](https://docs.rs/diener/) [![](https://img.shields.io/crates/v/diener.svg)](https://crates.io/crates/diener) [![](https://img.shields.io/crates/d/diener.png)](https://crates.io/crates/diener)

* [Usage](#usage)
* [License](#license)

### Usage

You can find the full documentation on [docs.rs](https://docs.rs/crate/diener).

#### Update

The `update` subcommand changes all `Cargo.toml` files in a given folder to use
a specific branch/path/commit/tag.

Change all Substrate dependencies in a folder to a different branch:

```rust
diener update --substrate --branch diener-branch
```

Or you want to change Polkadot and Substrate dependencies to the same branch:

```rust
diener update --branch diener-branch-2
```

Diener also supports `tag` and `rev` as arguments.

If a depdendency is belongs to Substrate or Polkadot is currently done by looking at the git url.
It also only works for repos called `substrate` or `polkadot`.

#### Patch

The `patch` subcommand adds a patch section for each crate in a given cargo workspace
to the workspace `Cargo.toml` file in some other cargo workspace.

Patch all Substrate git dependencies to be build from a given path:

```rust
diener patch --crates-to-patch ../path/to/substrate/checkout --substrate
```

This subcommand can be compared to `.cargo/config` without using a deprecated
feature of Cargo ;)

### License

Licensed under either of

 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)

 * [MIT license](http://opensource.org/licenses/MIT)

at your option.

License: Apache-2.0/MIT
