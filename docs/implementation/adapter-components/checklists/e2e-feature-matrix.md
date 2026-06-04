# E2E Feature Matrix Checklist

For every supported component feature, record the fixture id and harness test
that proves it.

| Axis      | Fixture id | Harness test | Visual assertion | Axe state | Notes |
| --------- | ---------- | ------------ | ---------------- | --------- | ----- |
| Pointer   |            |              |                  |           |       |
| Keyboard  |            |              |                  |           |       |
| Focus     |            |              |                  |           |       |
| State     |            |              |                  |           |       |
| Forms     |            |              |                  |           |       |
| Visual    |            |              |                  |           |       |
| A11y      |            |              |                  |           |       |
| Lifecycle |            |              |                  |           |       |

## Required Notes

- If an axis is unsupported, record `NotApplicable` and the reason in the
  matrix and PR body.
- If a counterpart feature is supported, it must appear in this matrix.
- If a visible state exists, it needs a computed visual assertion.
- If a state can be reached in the browser, axe must run in that state unless a
  scoped exception is documented.
