# Release Procedure

1. Stage environment.

    1. Fetch latest changes in `origin/master`.

        ```
        git fetch origin/master
        ```

    2. Checkout to `master`.

        ```
        git checkout master
        ```

    3. Ensure it is up to date with `origin/master` and there are no pending changes locally.

        ```
        git status
        ```

2. Verify current revision builds are green on [https://github.com/kafji/shub](https://github.com/kafji/shub).

3. Update documents.

    1. Bump version in [Cargo.toml](../Cargo.toml).

        ```toml
        version = "0.2.0"
        ```

    2. Update the changelog entries in [CHANGELOG.md](../CHANGELOG.md).

        ```
        ## [0.2.0](https://github.com/kafji/shub/tree/v0.2.0) - 2021-12-31

        - Add a feature

        ```

4. Commit changes.

    ```
    git commit -am "Prepare release 0.2.0"
    ```

5. Create tag.

    ```
    git tag v0.2.0
    ```

6. Publish the commit and the tag.

    ```
    git push origin master && git push --tags origin
    ```
