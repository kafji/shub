# Shub

Kafji's GitHub CLI.

## Installation

```bash
# install
cargo install --git https://github.com/kafji/shub
# set github credentials
export SHUB_USERNAME=kafji  # your github username
export SHUB_TOKEN=ghp_yourgithubaccesstoken  # your github personal access token
```

## Examples

### Delete all workflow runs in kafji/shub repository

```bash
shub actions delete-runs kafji/shub
```

### Download kafji/shub settings to a toml file

```bash
shub repos download-settings kafji/shub ./gh-repo-settings.toml
```

### Apply repository settings to GitHub repository

```bash
shub repos apply-settings ./gh-repo-settings.toml kafji/shub
```

### List starred repositories

```bash
shub stars --short
```

### List owned public not-archived repositories

```bash
shub repos list | grep public | grep -v archived
```

## License & Contribution

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
