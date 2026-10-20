import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import validate_t2_capture as validator


def fixture():
    phase = lambda name: {
        "name": name,
        "status": "observed",
        "bce_count": 1,
        "omitted": 0,
        "records": [{
            "bdf": "00:1f.0", "pci_id": "106b:1801",
            "command_status": "raw 0000/0000", "bars_0_5": ["raw 00000000"] * 6,
            "pm_msi": "PMCSR/MSI recorded", "evidence_ref": "video frame 10",
        }],
        "errors": [],
        "evidence_ref": "video frame 10",
    }
    return {
        "schema_version": 1,
        "run": {
            "model": "MacBookPro15,1", "firmware": "recorded firmware",
            "efi_sha256": "a" * 64, "config_sha256": "b" * 64,
            "topology": "fixed cable/port", "evidence_ref": "run video",
        },
        "phases": [phase(name) for name in validator.PHASES],
        "complete_marker": True,
    }


class CaptureValidatorTest(unittest.TestCase):
    def test_complete_marker_alone_cannot_hide_missing_phase(self):
        data = fixture()
        data["phases"][0].update(status="not_captured", bce_count=None, omitted=None, records=[], evidence_ref=None)
        missing = validator.validate(data)
        self.assertIn("EARLY NOT_CAPTURED", missing)
        self.assertIn("EARLY.evidence_ref NOT CAPTURED", missing)

    def test_omissions_and_error_lines_are_incomplete(self):
        data = fixture()
        data["phases"][1].update(bce_count=2, omitted=1, errors=["T2 CAP ERROR"])
        missing = validator.validate(data)
        self.assertTrue(any("omitted" in item for item in missing))
        self.assertTrue(any("error lines" in item for item in missing))

    def test_truncation_duplicate_phase_and_false_count_are_invalid(self):
        data = fixture()
        data["phases"][2]["records"][0]["bars_0_5"] = None
        self.assertTrue(validator.validate(data))
        data = fixture()
        data["phases"][2]["name"] = "EARLY"
        with self.assertRaises(validator.InvalidCapture):
            validator.validate(data)
        data = fixture()
        data["schema_version"] = True
        with self.assertRaises(validator.InvalidCapture):
            validator.validate(data)
        data = fixture()
        data["phases"][0]["records"].append(copy.deepcopy(data["phases"][0]["records"][0]))
        data["phases"][0].update(bce_count=2)
        data["phases"][0]["records"][1]["bdf"] = "00:1F.0"
        with self.assertRaises(validator.InvalidCapture):
            validator.validate(data)

    def test_identity_change_is_reported_as_incomplete(self):
        data = fixture()
        data["phases"][2]["records"][0]["pci_id"] = "106b:1802"
        self.assertIn("00:1f.0: PCI ID CHANGED across phases", validator.validate(data))
        data = fixture()
        data["phases"][2]["bce_count"] = 2
        with self.assertRaises(validator.InvalidCapture):
            validator.validate(data)

    def test_complete_schema_and_strict_cli(self):
        self.assertEqual(validator.validate(fixture()), [])
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "capture.json"
            data = fixture()
            data["phases"][0].update(status="not_captured", bce_count=None, omitted=None, records=[])
            path.write_text(json.dumps(data))
            self.assertEqual(validator.main([str(path), "--require-complete"]), 1)
            path.write_text('{"schema_version":1,"schema_version":1}')
            self.assertEqual(validator.main([str(path)]), 2)
            path.write_bytes(b"x" * (validator.MAX_BYTES + 1))
            self.assertEqual(validator.main([str(path)]), 2)


if __name__ == "__main__":
    unittest.main()
