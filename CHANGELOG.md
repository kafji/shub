# Changelog

## [Unreleased](https://github.com/kafji/shub/tree/master)

## [0.3.1](https://github.com/kafji/shub/tree/v0.3.1) - 2021-12-22

Changes:

- Add fork status in `repos list`

## [0.3.0](https://github.com/kafji/shub/tree/v0.3.0) - 2021-12-22

Breaking changes:

- Rename `stars` command to `starred`

Changes:

- Upgrade Rust to edition 2021
- Add `repos list` command to list owned repositories
- Un-re-export `github` module in `shub` crate
- Move `app` module into `shub`'s `lib` crate

## [0.2.0](https://github.com/kafji/shub/tree/v0.2.0) - 2021-09-04

Breaking changes:

- `stars` command
  - Remove URL column in
  - Print repository full name in first column instead of name with its organization name omitted

Changes:

- Add `short` switch for `stars` command to truncate long texts

## [0.1.1](https://github.com/kafji/shub/tree/v0.1.1) - 2021-08-25

Changes:

- Add new commad to print starred repositories.

## [0.1.0](https://github.com/kafji/shub/tree/v0.1.0) - 2021-08-25

Changes:

- Initial release. Check [README.md](README.md) for more information.

---

Release procedure handbook is at [docs/release-procedure.md](docs/release-procedure.md).
