import unittest
from with_macos_signing import select_identity

ONE = '  1) ' + 'A' * 40 + ' "Apple Development: Test Developer"\n     1 valid identities found\n'
TWO = ONE + '  2) ' + 'B' * 40 + ' "Developer ID Application: Test Developer"\n'


class SigningTests(unittest.TestCase):
    def test_single_identity_is_stable(self):
        self.assertEqual(select_identity(ONE, None, False), 'A' * 40)

    def test_multiple_identities_require_explicit_selection(self):
        with self.assertRaises(ValueError):
            select_identity(TWO, None, False)
        self.assertEqual(select_identity(TWO, 'Developer ID Application: Test Developer', False), 'B' * 40)
        self.assertEqual(select_identity(TWO, 'A' * 40, False), 'A' * 40)

    def test_missing_requested_identity_never_falls_back(self):
        with self.assertRaises(ValueError):
            select_identity(ONE, 'missing', True)

    def test_unsigned_fallback_requires_explicit_opt_in(self):
        for output, requested in [('', None), (ONE, '-')]:
            with self.assertRaises(ValueError):
                select_identity(output, requested, False)
            self.assertEqual(select_identity(output, requested, True), '-')


if __name__ == '__main__':
    unittest.main()
