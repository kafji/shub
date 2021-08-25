# Shub

Yet another, written in Rust, GitHub CLI.

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
shub stars
```
