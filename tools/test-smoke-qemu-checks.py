#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Reject missing/duplicate hubs even when aggregate totals look correct."""
import unittest
from smoke_qemu_checks import check_hub_enumerations


def log(discovery=5, live=5):
    def phase(name, count):
        return ('MUSHA: XHCI_' + name + ' BDF=0000000000000018\n'
                + 'MUSHA: USB2_HUB_READY\n' * count
                + 'MUSHA: USB_ENUMERATION_OK COUNT=0000000000000008\n')
    return phase('SCAN', discovery) + 'MUSHA: XHCI_SCAN_COMPLETE\n' + phase('INPUT', live)


class HubChecks(unittest.TestCase):
    def test_both_passes(self):
        for depth in range(1, 6):
            self.assertEqual(check_hub_enumerations(log(depth, depth), depth), [depth, depth])

    def test_missing_or_duplicate_in_either_pass(self):
        for counts in [(4, 5), (5, 4), (6, 5), (5, 6), (4, 6), (6, 4)]:
            with self.subTest(counts=counts), self.assertRaises(RuntimeError):
                check_hub_enumerations(log(*counts), 5)

    def test_missing_input(self):
        with self.assertRaises(RuntimeError):
            check_hub_enumerations(log().split('MUSHA: XHCI_INPUT')[0], 5)

    def test_repeated_pass(self):
        with self.assertRaises(RuntimeError):
            check_hub_enumerations(log() + log(), 5)

    def test_wrong_controller(self):
        with self.assertRaises(RuntimeError):
            check_hub_enumerations(log().replace('INPUT BDF=0000000000000018',
                                                 'INPUT BDF=0000000000000020'), 5)

    def test_missing_completion(self):
        with self.assertRaises(RuntimeError):
            check_hub_enumerations(log().replace('MUSHA: USB_ENUMERATION_OK COUNT=0000000000000008\n', '', 1), 5)

    def test_hubs_outside_completed_enumeration(self):
        for text in ['MUSHA: USB2_HUB_READY\n' + log(), log() + 'MUSHA: USB2_HUB_READY\n']:
            with self.assertRaises(RuntimeError):
                check_hub_enumerations(text, 5)

    def test_diagnostic_text_is_not_an_event(self):
        self.assertEqual(check_hub_enumerations(log() + 'MUSHA: DIAG USB2_HUB_READY\n', 5), [5, 5])


if __name__ == '__main__':
    unittest.main()
