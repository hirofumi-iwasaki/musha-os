#!/usr/bin/env python3
"""Validate a manually transcribed, bounded T2 trace capture; never touch hardware."""

import argparse
import json
import re
import sys
from pathlib import Path

MAX_BYTES = 65536
MAX_RECORDS = 64
PHASES = ("EARLY", "PRE_EBS", "POST_EBS")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
BDF = re.compile(r"[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-7]\Z")
PCI_ID = re.compile(r"[0-9a-fA-F]{4}:[0-9a-fA-F]{4}\Z")


class InvalidCapture(ValueError):
    pass


def _unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidCapture(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _keys(value, required, where):
    if not isinstance(value, dict) or set(value) != set(required):
        raise InvalidCapture(f"{where}: expected keys {sorted(required)}")


def _text(value, where, limit=256):
    if not isinstance(value, str) or not value or len(value) > limit:
        raise InvalidCapture(f"{where}: expected nonempty bounded text")


def _count(value, where):
    if type(value) is not int or not 0 <= value <= MAX_RECORDS:
        raise InvalidCapture(f"{where}: expected integer 0..{MAX_RECORDS}")


def validate(data):
    """Return missing-evidence reasons. Invalid data raises InvalidCapture."""
    _keys(data, ("schema_version", "run", "phases", "complete_marker"), "root")
    if type(data["schema_version"]) is not int or data["schema_version"] != 1:
        raise InvalidCapture("schema_version must be 1")
    run = data["run"]
    _keys(run, ("model", "firmware", "efi_sha256", "config_sha256", "topology", "evidence_ref"), "run")
    missing = []
    for field in ("model", "firmware", "topology", "evidence_ref"):
        if run[field] is None:
            missing.append(f"run.{field} NOT CAPTURED")
        else:
            _text(run[field], f"run.{field}")
    for field in ("efi_sha256", "config_sha256"):
        if run[field] is None:
            missing.append(f"run.{field} NOT CAPTURED")
        elif not isinstance(run[field], str) or not SHA256.fullmatch(run[field]):
            raise InvalidCapture(f"run.{field}: expected lowercase SHA-256")
    if type(data["complete_marker"]) is not bool:
        raise InvalidCapture("complete_marker must be boolean")
    if not data["complete_marker"]:
        missing.append("T2_DIAGNOSTICS_COMPLETE NOT CAPTURED")
    phases = data["phases"]
    if not isinstance(phases, list) or len(phases) != len(PHASES):
        raise InvalidCapture("phases must contain EARLY, PRE_EBS and POST_EBS exactly once")
    seen = set()
    identities = {}
    for phase in phases:
        _keys(phase, ("name", "status", "bce_count", "omitted", "records", "errors", "evidence_ref"), "phase")
        name = phase["name"]
        if not isinstance(name, str) or name not in PHASES or name in seen:
            raise InvalidCapture("unknown or duplicate phase")
        seen.add(name)
        status = phase["status"]
        if status not in ("observed", "not_captured", "error"):
            raise InvalidCapture(f"{name}: invalid status")
        if not isinstance(phase["records"], list) or len(phase["records"]) > MAX_RECORDS:
            raise InvalidCapture(f"{name}: records exceed bound")
        if not isinstance(phase["errors"], list) or len(phase["errors"]) > MAX_RECORDS:
            raise InvalidCapture(f"{name}: errors exceed bound")
        for error in phase["errors"]:
            _text(error, f"{name}.error")
        if phase["evidence_ref"] is None:
            missing.append(f"{name}.evidence_ref NOT CAPTURED")
        else:
            _text(phase["evidence_ref"], f"{name}.evidence_ref")
        if status != "observed":
            if phase["bce_count"] is not None or phase["omitted"] is not None or phase["records"]:
                raise InvalidCapture(f"{name}: unobserved phase cannot contain measured values")
            if status == "error" and not phase["errors"]:
                raise InvalidCapture(f"{name}: error provenance required")
            missing.append(f"{name} {status.upper()}")
            continue
        _count(phase["bce_count"], f"{name}.bce_count")
        _count(phase["omitted"], f"{name}.omitted")
        if len(phase["records"]) + phase["omitted"] != phase["bce_count"]:
            raise InvalidCapture(f"{name}: records + omitted differs from BCE count")
        if phase["omitted"]:
            missing.append(f"{name}: {phase['omitted']} BCE records omitted")
        if phase["errors"]:
            missing.append(f"{name}: error lines present")
        bdfs = set()
        for record in phase["records"]:
            _keys(record, ("bdf", "pci_id", "command_status", "bars_0_5", "pm_msi", "evidence_ref"), f"{name}.record")
            if not isinstance(record["bdf"], str) or not BDF.fullmatch(record["bdf"]):
                raise InvalidCapture(f"{name}: invalid BDF")
            bdf = record["bdf"].lower()
            if bdf in bdfs:
                raise InvalidCapture(f"{name}: duplicate BDF")
            bdfs.add(bdf)
            if not isinstance(record["pci_id"], str) or not PCI_ID.fullmatch(record["pci_id"]):
                raise InvalidCapture(f"{name}: invalid PCI ID")
            identities.setdefault(bdf, set()).add(record["pci_id"].lower())
            for field in ("command_status", "bars_0_5", "pm_msi", "evidence_ref"):
                if record[field] is None:
                    missing.append(f"{name} {record['bdf']}.{field} NOT CAPTURED")
                elif field == "bars_0_5":
                    if not isinstance(record[field], list) or len(record[field]) != 6:
                        raise InvalidCapture(f"{name} {record['bdf']}.bars_0_5: expected six raw BAR strings")
                    for bar in record[field]:
                        _text(bar, f"{name} {record['bdf']}.bar", 64)
                else:
                    _text(record[field], f"{name} {record['bdf']}.{field}", 1024)
    for bdf, ids in identities.items():
        if len(ids) > 1:
            missing.append(f"{bdf}: PCI ID CHANGED across phases")
    return missing


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args(argv)
    try:
        with args.capture.open("rb") as source:
            raw = source.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise InvalidCapture(f"file exceeds {MAX_BYTES} bytes")
        data = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_pairs)
        missing = validate(data)
    except (OSError, UnicodeError, json.JSONDecodeError, InvalidCapture, RecursionError) as exc:
        print(f"INVALID: {exc}", file=sys.stderr)
        return 2
    if missing:
        print("VALID SCHEMA; INCOMPLETE CAPTURE")
        for item in missing:
            print(f"- {item}")
        return 1 if args.require_complete else 0
    print("VALID STRUCTURE; THREE PHASES TRANSCRIBED (raw values and hardware acceptance unverified)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
