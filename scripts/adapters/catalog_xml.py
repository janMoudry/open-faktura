#!/usr/bin/env python3
"""User-configured XML -> import_prepare JSON. No native Money S4/S5 claim or network I/O."""
import argparse
import json
from pathlib import Path
import re
import uuid
import xml.etree.ElementTree as ET

FIELDS = {"key", "name", "unit_price", "currency", "vat_percent", "code", "barcode", "unit"}
REQUIRED = {"key", "name", "unit_price", "currency", "vat_percent"}
PATH = re.compile(r"[A-Za-z_][A-Za-z0-9_.-]*(?:/[A-Za-z_][A-Za-z0-9_.-]*)*")

def convert(raw: bytes, mapping: dict, agenda: str, request_id: str) -> dict:
    if len(raw) > 2_097_152 or b"<!DOCTYPE" in raw or b"<!ENTITY" in raw:
        raise ValueError("XML size/DTD/entity limit")
    text = raw.decode("utf-8-sig")
    if not isinstance(mapping, dict) or set(mapping) != {"record_path", "fields", "price_mode", "decimal_separator"}:
        raise ValueError("Mapping fields must be explicit")
    if mapping["price_mode"] not in ("net", "gross") or mapping["decimal_separator"] not in (".", ","):
        raise ValueError("Explicit price mode and decimal separator required")
    fields = mapping["fields"]
    if not isinstance(fields, dict) or not REQUIRED <= fields.keys() or fields.keys() - FIELDS:
        raise ValueError("Invalid field map")
    record_path = mapping["record_path"]
    if not isinstance(record_path, str) or not PATH.fullmatch(record_path):
        raise ValueError("record_path must be a simple relative element path")
    declaration = re.match(r'<\?xml[^?]*encoding=[\"\']([^\"\']+)', text, re.I)
    if declaration and declaration.group(1).upper() not in ("UTF-8", "UTF8"):
        raise ValueError("Only explicit UTF-8 XML is supported")
    root = ET.fromstring(text)
    pending, count = [(root, 1)], 0
    while pending:
        node, depth = pending.pop()
        count += 1
        if depth > 32 or count > 60_000:
            raise ValueError("XML depth/node limit")
        pending.extend((child, depth + 1) for child in node)
    if not agenda.strip() or len(agenda) > 200 or any(ord(c) < 32 for c in agenda):
        raise ValueError("Stable source agenda required")
    uuid.UUID(request_id)
    records, keys = [], set()
    for node in root.findall(record_path):
        if len(records) >= 500:
            raise ValueError("Maximum 500 records per preview")
        data = {"code": "", "barcode": "", "unit": "", "quantity": "1", "price_mode": mapping["price_mode"]}
        key = None
        for field, selector in fields.items():
            if isinstance(selector, dict) and set(selector) == {"constant"}:
                value = selector["constant"]
                if not isinstance(value, str):
                    raise ValueError("Constants must be strings")
            elif isinstance(selector, str):
                path, sep, attr = selector.rpartition("/@")
                if selector.startswith("@"):
                    path, sep, attr = "", "@", selector[1:]
                if sep:
                    if path and not PATH.fullmatch(path) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_.-]*", attr):
                        raise ValueError("Invalid attribute path")
                    matches = [node] if not path else node.findall(path)
                    if len(matches) != 1 or attr not in matches[0].attrib:
                        raise ValueError(f"Missing or ambiguous {field}")
                    value = matches[0].attrib[attr]
                else:
                    if not PATH.fullmatch(selector):
                        raise ValueError("Only direct element/attribute paths are supported")
                    matches = node.findall(selector)
                    if len(matches) != 1 or len(matches[0]):
                        raise ValueError(f"Missing or ambiguous leaf {field}")
                    value = matches[0].text or ""
            else:
                raise ValueError("Selector must be a path or explicit constant")
            value = value.strip()
            if field in REQUIRED and not value:
                raise ValueError(f"Missing {field}")
            if field == "key":
                key = value
            else:
                data[field] = value
        if key in keys or len(key) > 200 or any(ord(c) < 32 for c in key):
            raise ValueError("Duplicate or oversized stable key")
        keys.add(key)
        decimal_separator = re.escape(mapping["decimal_separator"])
        if not re.fullmatch(rf"\d+(?:{decimal_separator}\d{{1,4}})?", data["unit_price"]):
            raise ValueError("Price must be an exact decimal string, no thousands grouping")
        data["unit_price"] = data["unit_price"].replace(",", ".")
        if data["currency"] != "CZK" or data["vat_percent"] not in ("0", "12", "21"):
            raise ValueError("Unsupported catalog currency or VAT rate; do not infer a conversion")
        records.append({"kind": "catalog", "key": key, "data": data})
    if not records:
        raise ValueError("No records matched; verify the configured path")
    return {"request_id": request_id, "source": "generic", "agenda": agenda, "records": records}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--mapping", type=Path, required=True)
    parser.add_argument("--agenda", required=True)
    parser.add_argument("--request-id", default=None)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.input.stat().st_size > 2_097_152:
        parser.error("XML exceeds 2 MiB")
    try:
        result = convert(args.input.read_bytes(), json.loads(args.mapping.read_text()), args.agenda, args.request_id or str(uuid.uuid4()))
        # Exclusive creation prevents replacing a reviewed batch by accident.
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(result, output, ensure_ascii=False, indent=2)
            output.write("\n")
    except (ValueError, ET.ParseError, OSError) as error:
        parser.error(str(error))
if __name__ == "__main__":
    main()
