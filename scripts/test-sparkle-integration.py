#!/usr/bin/env python3
"""Native integration using a disposable, in-memory signing seed, never production keys."""
import base64
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
sdk = Path(sys.argv[1]).resolve()
binary = Path(sys.argv[2]).resolve()
root = Path(tempfile.mkdtemp(prefix="appdock-sparkle-test-"))
# Derive the public key with Apple's implementation; only the public key leaves the subprocess.
derive = root / "public-key.swift"
derive.write_text('''import CryptoKit
import Foundation
let seed = FileHandle.standardInput.readDataToEndOfFile()
let key = try Curve25519.Signing.PrivateKey(rawRepresentation: seed)
print(key.publicKey.rawRepresentation.base64EncodedString())
''')
seed = os.urandom(32)
public = subprocess.run(["xcrun", "swift", str(derive)], input=seed, capture_output=True, check=True).stdout.decode().strip()
env = dict(os.environ, SPARKLE_SDK=str(sdk), SPARKLE_PUBLIC_ED_KEY=public,
           SPARKLE_FEED_URL="https://example.invalid/appcast.xml", APPDOCK_PACKAGE_DIR=str(root / "package"))
subprocess.run(["scripts/package.sh", "--prebuilt", str(binary)], cwd=repo, env=env, check=True)
app = root / "package/AppDock.app"
# Do not touch the installed app's Sparkle preferences when smoke-testing the bundled updater.
info_path = app / "Contents/Info.plist"
info = plistlib.loads(info_path.read_bytes())
info["CFBundleIdentifier"] = "dev.appdock.updatefixture"
info["SUEnableAutomaticChecks"] = False
info_path.write_bytes(plistlib.dumps(info))
subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
version = info["CFBundleVersion"]
assets = root / "assets"
assets.mkdir()
archive = assets / f"AppDock-v{version}-macos-universal.dmg"
subprocess.run(["scripts/package-dmg.sh", str(app), str(archive)], cwd=repo, env=env, check=True)
subprocess.run(["scripts/verify-dmg.sh", str(archive), "universal"], cwd=repo, check=True)
env.update(SPARKLE_PRIVATE_KEY=base64.b64encode(seed).decode(), GITHUB_REPOSITORY="example/appdock")
subprocess.run(["scripts/generate-update-feed.sh", str(assets), f"v{version}"], cwd=repo, env=env, check=True)
# Check that the signature rejects even a one-byte archive change.
sys.path.insert(0, str(repo / "scripts"))
from validate_appcast import validate
url = f"https://github.com/example/appdock/releases/download/v{version}/{archive.name}"
signature = validate(assets / "appcast.xml", archive, version, url)
tampered = root / "tampered.dmg"
tampered.write_bytes(archive.read_bytes() + b"x")
result = subprocess.run(["xcrun", "swift", str(repo / "scripts/verify-update-signature.swift"), public, signature, str(tampered)], capture_output=True)
assert result.returncode != 0, "Tampered archive signature accepted"
wrong_public = base64.b64encode(bytes(32)).decode()
result = subprocess.run(["xcrun", "swift", str(repo / "scripts/verify-update-signature.swift"), wrong_public, signature, str(archive)], capture_output=True)
assert result.returncode != 0, "Mismatched release public key accepted"
print("Sparkle integration passed: universal app, nested signatures, signed feed, archive validation, tamper rejection, and mismatched-key rejection.")
print(f"Development fixture: {app}")
