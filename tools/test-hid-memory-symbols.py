#!/usr/bin/env python3
# Copyright 2026 Hirofumi Iwasaki
# SPDX-License-Identifier: Apache-2.0
import unittest
from hid_memory_symbols import body

HID = '_RNvMs0_NtCs79bTee2z47I_11musha_input3hidNtB5_6Layout5parse'
PAD = '_RNvMNtCs79bTee2z47I_11musha_input7gamepadNtB2_6Layout5parse'


def assembly(symbol, marker):
    return symbol + ':\n' + marker + '\n\t.def\tnext\n'


class Symbols(unittest.TestCase):
    def test_exact_hid_namespace_amid_gamepad(self):
        source = assembly(PAD, 'GAMEPAD') + assembly(HID, 'HID')
        self.assertEqual(body(source, '6Layout5parse', '11musha_input3hid'), 'HID')
        with self.assertRaises(AssertionError):
            body(source, '6Layout5parse')

    def test_missing_duplicate_and_prefix_are_rejected(self):
        for source in [assembly(PAD, 'PAD'), assembly(HID, 'A') + assembly(HID, 'B'),
                       assembly(HID.replace('3hidN', '3hidextraN'), 'PREFIX')]:
            with self.assertRaises(AssertionError):
                body(source, '6Layout5parse', '11musha_input3hid')

    def test_unmangled_probe_remains_unique(self):
        self.assertEqual(body(assembly('hid_probe_update', 'PROBE'), 'hid_probe_update'), 'PROBE')
        with self.assertRaises(AssertionError):
            body(assembly('hid_probe_update', 'A') * 2, 'hid_probe_update')


if __name__ == '__main__':
    unittest.main()
