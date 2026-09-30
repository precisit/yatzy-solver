"""The rules engine from Python (F7)."""

import pytest

import yatzy_solver as ys


def test_variants_and_codes():
    ids = ys.builtin_variants()
    assert "yatzy-scandinavian" in ids and "american" in ids and len(ids) == 12
    v = ys.Variant("american")
    assert v.name == "American rules (Yahtzee-compatible)"
    assert v.dice == 5 and v.rolls == 3 and len(v.categories) == 13
    assert v.num_action_codes == 13 + 210 and v.max_options == 13 + 31
    assert v.action_code("keep -") == 13 and v.action_notation(13 + 209) == "keep 6 6 6 6"
    assert v.action_code("score five_of_a_kind") == v.categories.index("five_of_a_kind")
    with pytest.raises(ValueError):
        ys.Variant("yahtzee")


def test_scoring_and_transitions():
    v = ys.Variant("yatzy-scandinavian")
    assert v.score("full_house", [3, 3, 3, 5, 5]) == 19
    assert v.score("two_pairs", [2, 2, 2, 2, 5]) == 0
    sit = v.start_turn("upper 0 | filled -", [6, 5, 3, 3, 1])
    assert sit == "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
    legal = v.legal_actions(sit)
    assert legal[0] == "score ones" and "keep 3 3" in legal and "keep 1 3 3 5 6" not in legal
    nxt = v.apply_keep(sit, [3, 3], [3, 4, 4])
    assert nxt == "dice 3 3 3 4 4 | rolls 1 | upper 0 | filled -"
    state, points = v.apply_score("upper 60 | filled ones,twos,fours,fives", [3, 3, 1, 2, 4], "threes")
    assert (state, points) == ("upper 66 | filled ones,twos,threes,fours,fives", 6 + 50)
    assert not v.is_over(state)
    with pytest.raises(ValueError):
        v.apply_keep(sit, [2], [1, 1, 1, 1])
    with pytest.raises(ValueError):
        v.legal_actions("dice 1 2 3 | rolls 2 | upper 0 | filled -")


def test_game():
    v = ys.Variant("american")
    g = ys.Game(v)
    r = g.score([5, 5, 5, 5, 5], "five_of_a_kind")
    assert r == {"points": 50, "upper_bonus": 0, "bonus": 0, "total": 50}
    r = g.score([5, 5, 5, 5, 5], "chance")
    assert r["bonus"] == 100 and g.bonus == 100
    assert g.state == "upper 0 | filled five_of_a_kind,chance | five_of_a_kind 50"
    assert g.points("chance") == 25 and g.points("ones") is None
    assert g.total == 175 and not g.is_over
    with pytest.raises(ValueError):
        g.score([1, 2, 3, 4, 5], "chance")
