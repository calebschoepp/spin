title = "Creating a new Spin release"
template = "spin_main"
date = "2023-07-11T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/release-process.md"

---

## Major / minor release

To cut a major / minor release of Spin, you will need to do the following:

1. Create a release branch, e.g. `v2.0`. With our branch protection rules this is easiest from the Github UI with the [New Branch button here](https://github.com/fermyon/spin/branches).

1. Switch to the release branch locally and update versions (e.g. `2.0.0-pre0` could be `2.0.0`).
   - Bump the version in Spin's `Cargo.toml`
   - Run `make build update-cargo-locks` so that `Cargo.lock` and example/test `Cargo.lock` files are updated

   PR these changes to the release branch ensuring that pull request has a base corresponding to the release branch (e.g. `v2.0`).

1. Create a new tag with a `v` and then the version number, e.g. `v2.0.0`. Then, push the tag to the `fermyon/spin` origin repo.

    As an example, via the `git` CLI:

    ```
    # Switch to the release branch
    git checkout v2.0
    git pull

    # Create a GPG-signed and annotated tag
    git tag -s -m "Spin v2.0.0" v2.0.0

    # Push the tag to the remote corresponding to fermyon/spin (here 'origin')
    git push origin v2.0.0
    ```

1. Switch back to `main` and update the `Cargo.toml` version again, this time to e.g. `2.1.0-pre0` if `2.1.0` is the next anticipated release.
   - Run `make build update-cargo-locks` so that `Cargo.lock` and example/test `Cargo.lock` files are updated
   - PR this to `main`
   - See [sips/011-component-versioning.md](sips/011-component-versioning.md)
     for details

Follow the [wrapping up](#wrapping-up) section to finish off the release process. 

## Patch release

To cut a patch release of Spin, you will need to do the following:

1. Backport the commits you wish to include to the release branch you're creating the patch release for. **NOTE** Use the [backport script](https://github.com/fermyon/spin/blob/main/.github/gh-backport.sh) to do so. 

```
$ ./.github/gh-backport.sh <pull-request> <branch-name>
```

1. Switch to the release branch locally and update versions (e.g. `2.0.0` could be `2.0.1`).
   - Bump the version in Spin's `Cargo.toml`
   - Run `make build update-cargo-locks` so that `Cargo.lock` and example/test `Cargo.lock` files are updated

   PR these changes to the release branch ensuring that pull request has a base corresponding to the release branch (e.g. `v2.0`).

1. Create a new tag with a `v` and then the version number, e.g. `v2.0.1`. Then, push the tag to the `fermyon/spin` origin repo.

    As an example, via the `git` CLI:

    ```
    # Switch to the release branch
    git checkout v2.0
    git pull

    # Create a GPG-signed and annotated tag
    git tag -s -m "Spin v2.0.1" v2.0.1

    # Push the tag to the remote corresponding to fermyon/spin (here 'origin')
    git push origin v2.0.1
    ```

    Follow the [wrapping up](#wrapping-up) section to finish off the release process.

## Release Candidate

To create a release candidate for a major/minor version of Spin, you will need to do the following:

1. Create the release branch if not already created. With our branch protection rules this is easiest from the Github UI with the [New Branch button here](https://github.com/fermyon/spin/branches). 
Otherwise, switch to the branch locally.

1. Update the Spin version with `-rc.N` where `N` is the release candidate number (e.g. `2.0.0-pre0` could be `2.0.0-rc.1`).
   - Bump the version in Spin's `Cargo.toml`
   - Run `make build update-cargo-locks` so that `Cargo.lock` and example/test `Cargo.lock` files are updated

   PR these changes to the release branch ensuring that pull request has a base corresponding to the release branch (e.g. `v2.0`).

1. Create a new tag with a `v` and then the version used above, e.g. `v2.0.0-rc.1`. Then, push the tag to the `fermyon/spin` origin repo.

    As an example, via the `git` CLI:

    ```
    # Switch to the release branch
    git checkout v2.0
    git pull

    # Create a GPG-signed and annotated tag
    git tag -s -m "Spin v2.0.0-rc.1" v2.0.0-rc.1

    # Push the tag to the remote corresponding to fermyon/spin (here 'origin')
    git push origin v2.0.0-rc.1
    ```

    Follow the [wrapping up](#wrapping-up) section to finish off the release process. 

## Wrapping up
1. Go to the GitHub [tags page](https://github.com/fermyon/spin/releases),
   edit the release and add the release notes. (This step is optional if a release candidate.)
   
1. Be sure to include instructions for
   [verifying the signed Spin binary](./sips/012-signing-spin-releases.md). The
   `--certificate-identity` value should match this release, e.g.
   `https://github.com/fermyon/spin/.github/workflows/release.yml@refs/tags/v2.0.0`.

1. Unless this is a release candidate, review and merge the bot-created Pull Request
   in the [fermyon/homebrew-tap repository](https://github.com/fermyon/homebrew-tap/).

The release is now complete!

[release action]: https://github.com/fermyon/spin/actions/workflows/release.yml
