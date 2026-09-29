# Working in yatzy-solver

The specification is `SPEC.md`. Build to it, milestone by milestone (section 8); each milestone has acceptance
criteria, and a milestone is done only when they pass.

## Workflow

- One branch per milestone (`m1-rules`, `m2-solver`, ...). Open a pull request into `main` when its acceptance
  criteria pass, with the evidence in the description (test output, the validation numbers, benchmark figures and
  the machine they were measured on). You may merge your own pull requests once CI is green.
- Keep commits focused, with messages that say what changed and why.
- Report at the end of each milestone: what was built, the acceptance evidence, open questions, and the next
  milestone.

## Correctness before speed

- The published values are the gate (SPEC 5.3 and M2): American rules 254.5896 (standard deviation 59.6117,
  median 248), Scandinavian Yatzy 248.63. If Scandinavian Yatzy does not reproduce 248.63, find and document the
  rule difference (SPEC 10.1) before tuning anything else.
- The brute-force reference solver for reduced games must agree exactly with the fast solver.
- Benchmarks on a laptop that is running other work are provisional: say so, and record the load.

## Stop and ask the owner before

- making the repository public, or publishing any package (crates.io, PyPI, npm);
- changing the license, the validation numbers or the acceptance criteria in `SPEC.md`;
- adding a dependency with a license other than MIT, Apache-2.0, BSD or similar permissive terms.

Spec changes go in `SPEC.md` in their own commits, with a changelog line; open questions go in its section 10.

## Writing

- The repository will become public: no hostnames, internal notes, secrets or personal paths in code, docs or
  commit messages.
- Plain punctuation in docs and messages: periods, commas and hyphens; no em or en dashes.
- Credit prior work (SPEC section 1) in the README.

See also `~/dev/AGENTS.md` for rules shared by all projects on this machine.
