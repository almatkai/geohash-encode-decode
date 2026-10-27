#!/usr/bin/env python3
"""Full beta Draft 2020-12 schema validation; requires jsonschema==4.23.0."""
import hashlib
import json
from pathlib import Path
from jsonschema import Draft202012Validator

root = Path(__file__).resolve().parents[1]
schema = json.loads((root / "tests/fixtures/beta/extension-v2.schema.json").read_text())
manifest = json.loads((root / "manifest.json").read_text())
Draft202012Validator.check_schema(schema)
Draft202012Validator(schema).validate(manifest)
icon = (root / "command-icon.png").read_bytes()
assert len(icon) == 172526
assert hashlib.sha256(icon).hexdigest() == "f59d02918ee4879a95940e3df317ff70ebeb901582967188b034b73daf0ee834"
print("Pinned beta schema validates; original icon hash and length match.")
