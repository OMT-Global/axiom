import unittest

from tests.helpers import PROGRAMS_DIR, assert_program_parity, load_program_fixture


class ConformanceTests(unittest.TestCase):
    def test_programs_match_expected_and_each_other(self) -> None:
        for path in sorted(PROGRAMS_DIR.glob("*.ax")):
            src, expected = load_program_fixture(path)
            assert_program_parity(self, src, expected, label=path.name)


if __name__ == "__main__":
    unittest.main()
