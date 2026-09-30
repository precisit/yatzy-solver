"""Exact solver, rules engine and data export for Scandinavian Yatzy and American rules (Yahtzee-compatible).

Situations and actions use the stable notation, for example::

    import yatzy_solver as ys

    solver = ys.Solver.build(ys.Variant("yatzy-scandinavian"))
    for action, value in solver.option_values("dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"):
        print(action, value)

Values are expected remaining scores (the score so far excluded). See the project documentation for the
notation, the action codes, the batch layout, the simulator's generator and the export schema.
"""

from ._yatzy_solver import RNG_VERSION, SOLVER_VERSION, Game, Solver, Variant, builtin_variants

__version__ = SOLVER_VERSION
__all__ = ["Game", "Solver", "Variant", "builtin_variants", "RNG_VERSION", "SOLVER_VERSION", "__version__"]
