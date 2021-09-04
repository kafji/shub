# Shub

GitHub CLI.

## Installation

```bash
# clone
git clone git@github.com:kafji/shub.git && cd ./shub/
# install
cargo install --path=.
# set github credentials
export SHUB_USERNAME=kafji  # your github username
export SHUB_TOKEN=ghp_yourgithubaccesstoken  # your github personal access token
```

## Commands

```bash
# delete all workflow runs in kafji/shub repository
shub actions delete-runs kafji/shub

# download kafji/shub settings to a toml file
shub repos download-settings kafji/shub ./gh-repo-settings.toml

# apply repository settings to GitHub repository
shub repos apply-settings ./gh-repo-settings.toml kafji/shub

# list starred repositories
shub stars --short
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
