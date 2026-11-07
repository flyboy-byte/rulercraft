# SPDX-License-Identifier: MIT OR Apache-2.0
"""The plan tool rejects the mistakes it exists to catch."""
import copy
import unittest
import plan


class PlanTests(unittest.TestCase):
    def setUp(self):
        self.roadmap, self.features = plan.load()

    def test_repository_plan_is_valid(self):
        self.assertEqual(plan.validate(self.roadmap, self.features), [])

    def bad(self, change):
        features = copy.deepcopy(self.features)
        change(features[0])
        self.assertTrue(plan.validate(self.roadmap, features))

    def test_rejects_bad_states(self):
        self.bad(lambda f: f.update(status="finished"))
        self.bad(lambda f: f.update(milestone="R9"))
        self.bad(lambda f: f.update(depends_on=["nope"]))
        self.bad(lambda f: f.update(status="verified", verified_by=["math"]))
        self.bad(lambda f: f.update(status="in-progress", owner=None))
        self.bad(lambda f: f.update(verified_by=["vibes"]))

    def test_progress_moves_with_status(self):
        _, before = plan.progress(self.roadmap, self.features)
        features = copy.deepcopy(self.features)
        for f in features:
            f["status"] = "done"
        _, after = plan.progress(self.roadmap, features)
        self.assertAlmostEqual(after, 1.0)
        self.assertLess(before, after)


if __name__ == "__main__":
    unittest.main()
