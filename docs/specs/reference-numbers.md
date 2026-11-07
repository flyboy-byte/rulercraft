# Local reference numbers

Status: *Outline*. Feature: `reference.local`.

Optional extra checks: another program's result for the same mesh and the same operation, captured on
Logan's machine into gitignored `fixtures/private/reference/<mesh>.json`. Tests compare RulerCraft's
numbers within the tolerance each test states, and skip with a message when the file is absent. The
program is never named in the repository; how to capture its numbers is written in gitignored `local/`.
Nothing from that program is linked, copied or committed (D-004).
