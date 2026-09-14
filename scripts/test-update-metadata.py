#!/usr/bin/env python3
import base64
import plistlib
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

from configure_updates import configure, validate
from validate_appcast import SPARKLE, validate as validate_appcast


class UpdateMetadataTests(unittest.TestCase):
    def test_rejects_missing_or_wrong_key_and_insecure_feed(self):
        public = base64.b64encode(bytes(32)).decode()
        for key in ["", "invalid", base64.b64encode(bytes(31)).decode()]:
            with self.assertRaises(ValueError):
                validate(key, "https://example.com/appcast.xml")
        for feed in ["http://example.com/appcast.xml", "file:///tmp/feed.xml", "https://user:password@example.com/feed", "https://example.com/feed#fragment"]:
            with self.assertRaises(ValueError):
                validate(public, feed)

    def test_bundle_security_settings_and_identity_survive(self):
        with tempfile.TemporaryDirectory() as root:
            bundle = Path(root) / "AppDock.app"
            path = bundle / "Contents/Info.plist"
            path.parent.mkdir(parents=True)
            original = {"CFBundleIdentifier": "dev.appdock.AppDock", "CFBundleVersion": "1.2.3"}
            path.write_bytes(plistlib.dumps(original))
            configure(bundle, base64.b64encode(bytes(32)).decode(), "https://example.com/appcast.xml")
            result = plistlib.loads(path.read_bytes())
            self.assertEqual(result["CFBundleIdentifier"], original["CFBundleIdentifier"])
            self.assertEqual(result["CFBundleVersion"], "1.2.3")
            self.assertTrue(result["SURequireSignedFeed"])
            self.assertTrue(result["SUVerifyUpdateBeforeExtraction"])
            self.assertFalse(result["SUAllowsAutomaticUpdates"])
            self.assertNotIn("SUEnableAutomaticChecks", result)

    def test_rejects_wrong_version_url_size_and_signature(self):
        with tempfile.TemporaryDirectory() as root:
            archive = Path(root) / "AppDock.dmg"
            archive.write_bytes(b"fixture-archive")
            feed = Path(root) / "appcast.xml"
            rss = ET.Element("rss")
            item = ET.SubElement(ET.SubElement(rss, "channel"), "item")
            version = ET.SubElement(item, SPARKLE + "version")
            version.text = "1.2.3"
            ET.SubElement(item, SPARKLE + "minimumSystemVersion").text = "12.0"
            url = "https://github.com/example/appdock/releases/download/v1.2.3/AppDock.dmg"
            enclosure = ET.SubElement(item, "enclosure", {"url": url, "length": str(archive.stat().st_size), SPARKLE + "edSignature": base64.b64encode(bytes(64)).decode()})
            def check():
                ET.ElementTree(rss).write(feed)
                return validate_appcast(feed, archive, "1.2.3", url)
            self.assertTrue(check())
            version.text = "1.2.2"
            with self.assertRaises(ValueError): check()
            version.text = "1.2.3"
            for key, value in [("url", "https://other.example/update"), ("length", "1"), (SPARKLE + "edSignature", "invalid")]:
                previous = enclosure.get(key)
                enclosure.set(key, value)
                with self.assertRaises(ValueError): check()
                enclosure.set(key, previous)


if __name__ == "__main__":
    unittest.main()
