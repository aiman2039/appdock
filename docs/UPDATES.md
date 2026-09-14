# Universal releases and in-app updates

AppDock releases contain one `AppDock-vVERSION-macos-universal.dmg` and an optional ZIP. Intel and Apple Silicon executables are built and tested on their respective native GitHub runners, combined with `lipo`, then packaged and signed once. The app and every embedded Sparkle helper contain both architectures.

Sparkle 2.10.0 is downloaded from its official release with a pinned SHA-256 digest. Its framework is embedded in the app, with its license. The unused sandbox XPC services are omitted; AppDock is not sandboxed. Signing proceeds from Sparkle's helpers to its framework to AppDock. The signed app and DMG are notarized and stapled before the final DMG is signed for Sparkle.

## One-time signing setup

Run these commands yourself in a normal macOS terminal. Do not send the private key in chat or commit it. Configure the publishing repository, `aiman2039/appdock`. GitHub currently redirects the old `ohaddahan/appdock` origin to this repository.

1. Download the pinned SDK and generate an AppDock signing key in your login Keychain. This reuses an existing key for the same account rather than replacing it:

   ```sh
   scripts/fetch-sparkle.sh
   target/sparkle/Sparkle-2.10.0/bin/generate_keys --account appdock
   ```

2. Set the **public** key as a repository variable (use a different repository only if you intentionally publish there):

   ```sh
   target/sparkle/Sparkle-2.10.0/bin/generate_keys --account appdock -p |
     gh variable set SPARKLE_PUBLIC_ED_KEY --repo aiman2039/appdock
   ```

3. In **Keychain Access**, locate “Private key for signing Sparkle updates” with account `appdock`. Copy its password, run the following command, and paste it into gh's hidden prompt:

   ```sh
   gh secret set SPARKLE_PRIVATE_KEY --repo aiman2039/appdock
   ```

The private key stays in Keychain and the GitHub secret. CI sends it to Sparkle through stdin; it is never written to a temporary key file or passed on the command line. Keep the same key for future releases. Copy both settings to another publishing repository only if you intend it to use the same update-signing identity.

The release workflow validates both settings before creating the release tag. It verifies the update signature against the public key embedded in the app, so mismatched keys fail the build.

## Publishing

Bump and commit a stable `MAJOR.MINOR.PATCH` Cargo version, then manually run **Release**. The new stable update feed currently excludes prerelease version formats; validation rejects those before tagging. No signing, publishing, or deployment happens from a local build.

Each GitHub Release includes a signed `appcast.xml`. AppDock's embedded feed URL is:

```text
https://github.com/OWNER/REPO/releases/latest/download/appcast.xml
```

The owner/repository is taken from the workflow's repository, not hardcoded to a fork. Feed entries point to the versioned universal DMG in that same release. The feed and assets are uploaded before the draft becomes public. Existing releases before this integration do not contain Sparkle: users must install the first Sparkle-enabled release manually once.

A feed contains the current stable update only, without binary deltas. Future minimum-OS changes or key rotation need a deliberate migration plan that preserves an update route for older installations. Do not make an older version the latest release or change the signing key casually.

## User experience and window restoration

After onboarding, Sparkle starts its normal update-check permission flow. Users can use **AppDock → Check for Updates…** at any time, and toggle **Automatically Check for Updates**. Sparkle's standard update UI handles download and installation. The user approves installation; unattended installation is disabled.

Before an update restarts AppDock, Sparkle's relaunch callback is deferred. The existing worker restores managed windows and persists the workspace first. A failed restoration keeps the update waiting while the user retries or keeps AppDock open. The callback resumes only after the worker reports a successful shutdown.

The app requires signed feeds and verifies update signatures before extraction. System profiling is disabled. Local `cargo run` builds work without downloading or loading Sparkle, and explain that updates require a packaged release.

## Validation

`./scripts/check.sh` covers Rust behavior and update metadata validation. `scripts/test-sparkle-integration.py SDK_PATH UNIVERSAL_BINARY` exercises real Sparkle feed signing, mounted universal packaging, tamper rejection, and mismatched-key rejection with a disposable in-memory seed. It leaves only a public-key development fixture for inspection. Run its packaged app with an isolated `APPDOCK_DATA_DIR` and `--updater-smoke` to check native framework startup and the failed-save/retry relaunch barrier (the fixture must contain a binary built with that flag). Release packaging mounts the final DMG read-only and checks the bundled app's signature, both architectures, Sparkle helper architectures, and Applications shortcut. `generate-update-feed.sh` uses Sparkle's official feed generator, verifies feed metadata and archive size, checks the archive signature with Apple's CryptoKit against the embedded public key, and verifies the signed feed with Sparkle.

Local development installers are ad-hoc signed and are not production release evidence. Test an actual update between two Developer ID signed, notarized releases on a separate Mac before considering the hosted installation/relaunch path verified.

References: [Sparkle integration](https://sparkle-project.org/documentation/), [manual signing](https://sparkle-project.org/documentation/sandboxing/#code-signing), [update publishing](https://sparkle-project.org/documentation/publishing/), [universal macOS binaries](https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary).
