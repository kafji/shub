# Shub

Kafji's workspace CLI.

## Install

```bash
# install
cargo install --git 'ssh://git@github.com/kafji/shub.git'
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

### List starred repositories written in language other than Kotlin

```bash
shub stars --short --lang '!kotlin'
```

### List owned public not-archived repositories

```bash
shub repos list | grep public | grep -v archived
```
