# Homebrew distribution

`trickle.rb` is the cask definition. It does not live in this repository's
install path — Homebrew reads casks from a tap, so the file has to be copied
into one.

## Creating the tap, once

A tap is just a GitHub repository whose name starts with `homebrew-`:

1. Create `swsususu/homebrew-tap` on GitHub.
2. Add the cask at `Casks/trickle.rb` (the `Casks/` directory is required).

```bash
git clone git@github.com:swsususu/homebrew-tap.git
cd homebrew-tap
mkdir -p Casks
cp /path/to/Trickle/packaging/homebrew/trickle.rb Casks/
git add Casks/trickle.rb
git commit -m "Add Trickle cask"
git push
```

Users then install with:

```bash
brew tap swsususu/tap
brew install --cask trickle
```

Or in one step: `brew install --cask swsususu/tap/trickle`.

## Per-release update

The cask pins a checksum, so it needs editing on every release. `:no_check`
would avoid this but silently accepts a tampered download.

1. Publish the GitHub release so the DMG asset is downloadable.
2. Compute its checksum:

```bash
curl -fsSL -o /tmp/trickle.dmg \
  "https://github.com/swsususu/Trickle/releases/download/v0.0.1/Trickle_0.0.1_universal.dmg"
shasum -a 256 /tmp/trickle.dmg
```

3. Update `version` and `sha256` in `Casks/trickle.rb`, then push.

The asset name comes from `productName` in `tauri.conf.json` and the CI target,
currently producing `Trickle_<version>_universal.dmg`. If either changes, the
`url` in the cask has to follow.

## Verifying before pushing

```bash
brew audit --cask --online Casks/trickle.rb
brew style Casks/trickle.rb
brew install --cask Casks/trickle.rb   # installs from the local file
brew uninstall --cask trickle
```

`brew audit --online` is what catches a wrong checksum or a dead URL, so run it
against the published release rather than trusting the file.

## On the official homebrew-cask

Submitting there is possible but not immediate: it expects a project with some
adoption, tagged stable releases, and an identity of its own rather than a fork.
A personal tap has none of those requirements and works identically for users
who add it.

## Notarisation

The app is ad-hoc signed, not notarised, so Gatekeeper refuses it on first
launch. **Homebrew does not work around this.** Installing via `brew` sets the
"user approved" bit on the quarantine attribute, but macOS still blocks an app
whose signature it cannot verify — verified by installing this cask and getting
the "Apple could not verify" dialog. The cask therefore ships a `caveats` block
telling users to right-click and Open, or to clear the attribute:

```bash
xattr -dr com.apple.quarantine /Applications/Trickle.app
```

`spctl -a -vvv -t exec /Applications/Trickle.app` reports `rejected` either way;
that is the notarisation status, and only a 99 USD/year Apple Developer account
changes it. With one, `APPLE_CERTIFICATE` and friends in the CI workflow start
producing signed builds, and the caveats block can be dropped.

So the honest reason to offer a tap is convenience and upgrades
(`brew upgrade --cask trickle`), not bypassing Gatekeeper.
