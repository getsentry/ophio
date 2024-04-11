from sentry_ophio.ketama import KetamaPool


def test_hasher():
    pool = KetamaPool(["a"])
    assert pool.get_slot("a") == 0
    assert pool.get_slot("b") == 0

    pool = KetamaPool(["a", "b", "c", "d", "e"])

    # these here are pretty random depending on the hashing state
    assert pool.get_slot("a") == 4
    assert pool.get_slot("b") == 3
    assert pool.get_slot("c") == 3

