"""Cross-language parity (SPEC 5.3): the golden set, bit for bit, through the single and the batch API."""

import json

import numpy as np

from conftest import GOLDEN


def golden():
    return [json.loads(line) for line in GOLDEN.read_text().splitlines()]


def test_golden_single(solvers):
    rows = golden()
    assert len(rows) == 300
    for r in rows:
        s = solvers[r["variant"]]
        v = s.variant
        got = s.option_values(r["situation"])
        assert [v.action_code(a) for a, _ in got] == [c for c, _ in r["options"]]
        # Exact equality: the same f64, not merely close.
        assert [x for _, x in got] == [x for _, x in r["options"]]
        state = r["situation"].split(" | ", 2)[2]
        assert s.state_value(state) == r["state_value"]


def test_golden_batch_and_dense(solvers):
    rows = golden()
    for vid, s in solvers.items():
        v = s.variant
        mine = [r for r in rows if r["variant"] == vid]
        arr = v.situation_arrays([r["situation"] for r in mine])
        codes, values, counts = s.option_values_batch(arr["filled"], arr["upper"], arr["armed"], arr["dice"], arr["rolls_left"])
        assert codes.shape == values.shape == (len(mine), v.max_options)
        assert codes.dtype == np.int16 and values.dtype == np.float64 and counts.dtype == np.uint16
        dense = v.dense(codes, values, counts)
        assert dense.shape == (len(mine), v.num_action_codes)
        for i, r in enumerate(mine):
            n = len(r["options"])
            assert counts[i] == n
            assert codes[i, :n].tolist() == [c for c, _ in r["options"]]
            assert values[i, :n].tolist() == [x for _, x in r["options"]]
            assert (codes[i, n:] == -1).all() and np.isnan(values[i, n:]).all()
            for c, x in r["options"]:
                assert dense[i, c] == x
            assert np.count_nonzero(~np.isnan(dense[i])) == n
        states = s.state_values(arr["filled"], arr["upper"], arr["armed"])
        assert states.tolist() == [r["state_value"] for r in mine]
