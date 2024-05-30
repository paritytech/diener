# diener - dependency diener is a tool for easily changing [Polkadot SDK](https://github.com/paritytech/polkadot-sdk) dependency versions

[![](https://docs.rs/diener/badge.svg)](https://docs.rs/diener/) [![](https://img.shields.io/crates/v/diener.svg)](https://crates.io/crates/diener) [![](https://img.shields.io/crates/d/diener.png)](https://crates.io/crates/diener)

* [Usage](#usage)
* [License](#license)

### Usage

You can find the full documentation on [docs.rs](https://docs.rs/crate/diener).

#### Update

The `update` subcommand changes all `Cargo.toml` files in a given folder to use
a specific branch/path/commit/tag.

Change all Polkadot SDK dependencies in a folder to a different branch:

```rust
diener update --branch diener-branch
```

Diener also supports `tag` and `rev` as arguments.

#### Patch

The `patch` subcommand adds a patch section for each crate in a given cargo workspace
to the workspace `Cargo.toml` file in some other cargo workspace.

Patch all git dependencies to be build from a given path:

```rust
diener patch --crates-to-patch ../path/to/polkadot-sdk/checkout
```

This subcommand can be compared to `.cargo/config` without using a deprecated
feature of Cargo ;)

### License

Licensed under either of

 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)

 * [MIT license](http://opensource.org/licenses/MIT)

at your option.

License: Apache-2.0/MIT
