"""Queries, simulation and export from Python."""

import json

import numpy as np
import pytest

import yatzy_solver as ys


def test_queries(solvers):
    s = solvers["yatzy-scandinavian"]
    assert s.precision == "f64" and s.tie_epsilon == 1e-9
    assert round(s.state_value("upper 0 | filled -"), 7) == 248.4399894
    sit = "dice 1 1 2 2 3 | rolls 2 | upper 63 | filled ones,twos,threes,fours,fives,sixes,one_pair,two_pairs,three_of_a_kind,four_of_a_kind,small_straight,large_straight,full_house,chance"
    assert [a for a, _ in s.best_options(sit)] == ["keep 1 1", "keep 2 2"]
    assert s.best_action(sit) == "keep 1 1"
    assert s.regret(sit, "keep 2 2") == 0.0 and s.regret(sit, "keep 3") > 0
    with pytest.raises(ValueError):
        s.regret(sit, "score chance")


def test_simulate_optimal_and_python_policy(solvers):
    s = solvers["american"]
    opt = s.simulate(200, seed=5, logs=True)
    assert opt["scores"].dtype == np.uint16 and len(opt["scores"]) == 200 and opt["rng"] == 1
    assert opt["logs"][0][0] == "rng 1 | seed 5 | game 0"
    # A Python policy that plays the best action reproduces the optimal games exactly.
    mine = s.simulate(200, seed=5, logs=True, policy=lambda sit, legal, score: s.best_action(sit))
    assert (mine["scores"] == opt["scores"]).all() and mine["logs"] == opt["logs"]
    # A random policy sees the same first roll of every turn (common random numbers).
    import random

    rnd = random.Random(1)
    other = s.simulate(20, seed=5, logs=True, policy=lambda sit, legal, score: rnd.choice(legal))
    first = lambda log: [l.split(" | ")[0] for l in log if " | rolls 2 | " in l]
    for a, b in zip(opt["logs"][:20], other["logs"]):
        assert first(a) == first(b)
    assert other["mean"] < 150


def test_policy_errors_propagate(solvers):
    s = solvers["american"]

    def boom(sit, legal, score):
        raise RuntimeError("policy failed")

    with pytest.raises(RuntimeError, match="policy failed"):
        s.simulate(3, policy=boom)
    with pytest.raises(ValueError):
        s.simulate(3, policy=lambda sit, legal, score: "score nothing")


def test_export_jsonl_and_parquet(solvers, tmp_path):
    s = solvers["yatzy-scandinavian"]
    p = tmp_path / "rows.jsonl"
    assert s.export(str(p), 500, source="perturbed", seed=3, perturb=0.2) == 500
    rows = [json.loads(l) for l in p.read_text().splitlines()]
    assert len(rows) == 500 and rows[0]["source"] == "perturbed" and rows[0]["perturb"] == 0.2
    assert all(isinstance(r["random"], bool) for r in rows) and any(r["random"] for r in rows)
    for r in rows[:50]:
        got = s.option_values(r["notation"])
        assert [x for _, x in got] == [o["value"] for o in r["options"]]
        assert r["best_value"] == max(o["value"] for o in r["options"])
    pq = pytest.importorskip("pyarrow.parquet")
    q = tmp_path / "rows.parquet"
    assert s.export(str(q), 500, source="uniform", seed=3, format="parquet") == 500
    t = pq.read_table(q)
    assert t.num_rows == 500 and t.column_names[-2:] == ["chosen", "random"]
    first = t.slice(0, 1).to_pylist()[0]
    assert first["score_so_far"] is None and first["chosen"] is None and first["random"] is None
    got = s.option_values(first["notation"])
    assert [x for _, x in got] == [o["value"] for o in first["options"]]
