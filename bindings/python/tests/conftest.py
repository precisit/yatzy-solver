import pathlib

import pytest

import yatzy_solver as ys

GOLDEN = pathlib.Path(__file__).resolve().parents[3] / "golden" / "parity.jsonl"


@pytest.fixture(scope="session")
def solvers():
    return {vid: ys.Solver.build(ys.Variant(vid)) for vid in ("yatzy-scandinavian", "american")}
