"""Embed release update settings. Never accepts private key material."""
import base64
import os
import plistlib
import re
import tomllib
import sys
from pathlib import Path
from urllib.parse import urlparse


def validate(public_key, feed):
    try:
        key = base64.b64decode(public_key, validate=True)
    except ValueError as error:
        raise ValueError("SPARKLE_PUBLIC_ED_KEY must be a base64 public key") from error
    if len(key) != 32:
        raise ValueError("SPARKLE_PUBLIC_ED_KEY must decode to 32 bytes")
    url = urlparse(feed)
    if url.scheme != "https" or not url.hostname or url.username or url.password or url.fragment:
        raise ValueError("Sparkle feed must be an HTTPS URL without credentials or fragment")


def configure(bundle, public_key, feed):
    validate(public_key, feed)
    path = bundle / "Contents/Info.plist"
    with path.open("rb") as file:
        info = plistlib.load(file)
    info.update(
        SUFeedURL=feed,
        SUPublicEDKey=public_key,
        SUVerifyUpdateBeforeExtraction=True,
        SURequireSignedFeed=True,
        SUAllowsAutomaticUpdates=False,
        SUEnableSystemProfiling=False,
    )
    # Retain Sparkle's user choice for automatic checks (default prompt on second launch).
    with path.open("wb") as file:
        plistlib.dump(info, file)


if __name__ == "__main__":
    try:
        key = os.environ.get("SPARKLE_PUBLIC_ED_KEY", "")
        feed = os.environ.get("SPARKLE_FEED_URL", "")
        if sys.argv[1] == "--check":
            validate(key, feed)
            with open("Cargo.toml", "rb") as manifest:
                version = tomllib.load(manifest)["package"]["version"]
            if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
                raise ValueError("The stable Sparkle release feed requires a MAJOR.MINOR.PATCH version")
            if os.environ.get("HAS_SPARKLE_PRIVATE_KEY") == "false":
                raise ValueError("Missing SPARKLE_PRIVATE_KEY repository secret; see docs/UPDATES.md")
        else:
            configure(Path(sys.argv[1]), key, feed)
    except (ValueError, OSError) as error:
        sys.exit(str(error))
