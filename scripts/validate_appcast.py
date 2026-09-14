"""Validate generated feed metadata before upload; print only the public signature."""
import base64
import sys
from pathlib import Path
import xml.etree.ElementTree as ET

SPARKLE = "{http://www.andymatuschak.org/xml-namespaces/sparkle}"


def validate(feed, archive, version, url):
    root = ET.parse(feed).getroot()
    items = root.findall("./channel/item")
    if len(items) != 1:
        raise ValueError("Expected one current update in the feed")
    item = items[0]
    enclosure = item.find("enclosure")
    if enclosure is None:
        raise ValueError("Missing update enclosure")
    if item.findtext(SPARKLE + "version") != version:
        raise ValueError("Update version does not match release tag")
    if enclosure.get("url") != url:
        raise ValueError("Update URL does not match this release")
    if int(enclosure.get("length", "0")) != archive.stat().st_size:
        raise ValueError("Update length does not match archive")
    signature = enclosure.get(SPARKLE + "edSignature", "")
    if len(base64.b64decode(signature, validate=True)) != 64:
        raise ValueError("Missing or invalid Ed25519 signature")
    if item.findtext(SPARKLE + "minimumSystemVersion") != "12.0":
        raise ValueError("Unexpected minimum macOS version in update feed")
    return signature


if __name__ == "__main__":
    try:
        print(validate(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4]))
    except (ValueError, OSError, ET.ParseError) as error:
        sys.exit(str(error))
