# Distributing Showcase

This guide is for **maintainers**. It explains how releases are built and how to
enable the optional signed APT repository so users can `apt install showcase`
and get automatic updates.

## How releases work

Releases are fully automated by [`.github/workflows/release.yml`](../.github/workflows/release.yml).

1. Bump the version in `src-tauri/tauri.conf.json` (and `package.json`) and commit.
2. Tag the commit and push the tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

3. The `Release` workflow (any tag matching `v*`) then:
   - builds the `.deb` and `.AppImage` with `npm run tauri build -- --bundles deb,appimage`,
   - collects them into `dist/` and writes a `SHA256SUMS` checksum file,
   - generates [build provenance attestations](https://docs.github.com/actions/security-guides/using-artifact-attestations) for the binaries,
   - publishes a **GitHub Release** for the tag with all of `dist/*` attached and auto-generated release notes.

No secrets are required for the release build itself.

## One-time setup: enable the signed APT repository

The APT repo is **opt-in**. Until you complete these steps the
[`apt-repo.yml`](../.github/workflows/apt-repo.yml) workflow no-ops (it is
guarded by the `APT_REPO_ENABLED` repo variable), so nothing breaks if the
signing secrets are absent.

### 1. Generate a dedicated signing GPG key

Use a **dedicated** key for repository signing (not your personal key).

Quick, non-interactive (recommended):

```bash
gpg --quick-generate-key "Showcase Apt Signing <rabiulislamemon@gmail.com>" rsa4096 sign 2y
```

This creates a 4096-bit RSA key valid for 2 years, usable only for signing.
You will be prompted for a passphrase (you may leave it empty for fully
unattended CI, but a passphrase stored as a secret is preferred).

Or, for the full interactive flow:

```bash
gpg --full-generate-key
# Choose: (4) RSA (sign only); 4096 bits; expiry 2y;
# Real name: Showcase Apt Signing; Email: rabiulislamemon@gmail.com
```

Find the key ID (the long hex fingerprint or the `sec` line):

```bash
gpg --list-secret-keys --keyid-format=long "Showcase Apt Signing"
```

### 2. Store the signing secrets in the repo

Export the **private** key and store it as a GitHub Actions secret, plus its
passphrase:

```bash
gpg --armor --export-secret-keys <KEYID> | gh secret set GPG_PRIVATE_KEY
gh secret set GPG_PASSPHRASE          # paste the passphrase; leave empty if none
```

> If the key has no passphrase, you can set `GPG_PASSPHRASE` to an empty value;
> the workflow handles an empty passphrase.

### 3. Enable GitHub Pages

In the repo on GitHub: **Settings → Pages → Build and deployment → Source =
"GitHub Actions"**. The APT repo is published to
<https://rabiulislam-xyz.github.io/showcase>.

### 4. Flip the opt-in variable

```bash
gh variable set APT_REPO_ENABLED --body true
```

This is what makes `apt-repo.yml` actually run. With it unset (or not `true`)
the workflow is skipped.

### 5. Cut a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release build runs first; when the GitHub Release is **published**, the
`Publish APT repository` workflow downloads the release `.deb` assets, builds a
signed `stable/main` apt repo, and deploys it to GitHub Pages.

## What gets published

- The signed repository tree (`pool/`, `dists/stable/...`) at the Pages URL.
- The **public** signing key, dearmored for `/etc/apt/keyrings`, at:

  <https://rabiulislam-xyz.github.io/showcase/showcase-archive-keyring.gpg>

Users add the repo with the commands in the project [README](../README.md#b-apt-repository-auto-updates).

## Key rotation / renewal

When the key nears expiry (or you need to rotate), generate a new key, repeat
steps 1–2 to update `GPG_PRIVATE_KEY`/`GPG_PASSPHRASE`, and re-run the workflow
(re-publish a release or trigger `apt-repo.yml` via **workflow_dispatch**). The
new public key is republished automatically; existing users pick it up on their
next `apt update` because the keyring file at the Pages URL is overwritten.

## Manual / local repo build

You can build the repo locally for testing (the signing key must be in your
keyring):

```bash
scripts/build-apt-repo.sh path/to/debs ./public
```

See [`scripts/build-apt-repo.sh`](../scripts/build-apt-repo.sh) for the exact
layout it produces.
