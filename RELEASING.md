# Releasing

A release publishes `problem-details-fine` and `problem-details-fine-macros` to crates.io.
[release-plz](https://release-plz.dev) does the work. The `Release` workflow in
`.github/workflows/release.yml` runs it. The configuration is in `release-plz.toml`.

## What a release contains

- Both crates get the same version. They are in one version group, `problem-details-fine`.
- The repository has one changelog, `CHANGELOG.md` at the root. It lists the changes to both crates under the
  version of `problem-details-fine`. The macros crate reaches users through the main crate.
- The commit gets the tag `vX.Y.Z`, and GitHub gets a release `vX.Y.Z`. The body of the GitHub release is the
  changelog entry.
- `problem-details-fine-macros` gets no tag and no GitHub release of its own.

release-plz writes the changelog from the commit messages. Write commit messages in the
[Conventional Commits](https://www.conventionalcommits.org) format, for example `fix: ...` or `build(ci): ...`.

## Before the first release

The `release` job publishes with the `CARGO_REGISTRY_TOKEN` secret. If the secret is not set, the job uses
crates.io trusted publishing.

crates.io accepts trusted publishing only for a crate that already exists. Thus, for the first release:

1. Make a crates.io API token with the `publish-new` and `publish-update` scopes.
2. Add it as the repository secret `CARGO_REGISTRY_TOKEN`.

After the first release, you can configure trusted publishing for both crates on crates.io, and then remove the
secret.

## Release steps

1. Make sure that CI passes on `main`.
2. Run the `Release` workflow with the command `release-pr`. Use the Actions tab, or:

   ```sh
   gh workflow run release.yml -f command=release-pr
   ```

   release-plz opens a pull request. The pull request changes the version and the changelog.
3. Review the pull request. Check the new version and the changelog entry. Edit the changelog in the pull request
   if necessary.
4. Start CI on the pull request. GitHub does not start workflows for a pull request that `GITHUB_TOKEN` opens.
   Close the pull request and open it again, or push a commit to its branch.
5. Merge the pull request when CI passes.
6. Run the `Release` workflow with the command `release`:

   ```sh
   gh workflow run release.yml -f command=release
   ```

   release-plz publishes both crates, tags the commit, and makes the GitHub release.
7. Check the result:
   - Both crates show the new version on crates.io.
   - The tag `vX.Y.Z` and the GitHub release exist.
   - docs.rs builds the documentation of the new version.

## If the `release` job fails

release-plz skips a crate version that is already on crates.io. Thus, after you fix the cause, run the `release`
command again. Do not change the version by hand.

A version on crates.io is permanent. To withdraw a bad version, yank it with `cargo yank`, and release a fix.
