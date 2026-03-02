from __future__ import annotations

import unittest

from axiom.intops import trunc_div


class IntOpsTests(unittest.TestCase):
    def test_division_trunc_toward_zero(self) -> None:
        self.assertEqual(trunc_div(3, 2), 1)
        self.assertEqual(trunc_div(-3, 2), -1)
        self.assertEqual(trunc_div(3, -2), -1)
        self.assertEqual(trunc_div(-3, -2), 1)
