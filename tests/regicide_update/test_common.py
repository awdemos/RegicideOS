import unittest
from regicide_update import common as rc


class CommonTests(unittest.TestCase):
    def test_colours(self):
        self.assertTrue(rc.Colours.red.startswith("\033["))

    def test_overlay_subvolumes(self):
        # Gentoo keeps /usr on the immutable ROOTS partition (dracut switch-root
        # breaks with a separate /usr mount), so only etc/var are snapshotted.
        # The Arch variant snapshots ("etc", "var", "usr").
        self.assertEqual(rc.OVERLAY_SUBVOLUMES, ("etc", "var"))


if __name__ == "__main__":
    unittest.main()
