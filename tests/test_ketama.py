from sentry_ophio.ketama import KetamaPool


def test_hasher():
    pool = KetamaPool(["a"])
    assert pool.get_node("a") == "a"
    assert pool.get_node("b") == "a"

    pool = KetamaPool(["a", "b", "c", "d", "e"])

    # these here are pretty random depending on the hashing state
    assert pool.get_node("a") == "e"
    assert pool.get_node("b") == "d"
    assert pool.get_node("c") == "d"


def test_consistent_hashing():
    pool = KetamaPool(["node-a", "node-b", "node-c", "node-d", "node-e"])

    assert pool.get_node("key-a") == "node-e"
    assert pool.get_node("key-b") == "node-d"
    assert pool.get_node("key-c") == "node-b"
    assert pool.get_node("key-d") == "node-a"
    assert pool.get_node("key-e") == "node-e"
    assert pool.get_node("key-aa") == "node-b"

    pool.add_node("node-f")

    # most existing keys are unchanged
    assert pool.get_node("key-a") == "node-e"
    assert pool.get_node("key-b") == "node-d"
    assert pool.get_node("key-c") == "node-b"
    assert pool.get_node("key-d") == "node-a"
    assert pool.get_node("key-e") == "node-e"
    # one key has moved to the new node
    assert pool.get_node("key-aa") == "node-f" # <-

    pool.remove_node("node-f")

    # we are back to the original assignment
    assert pool.get_node("key-a") == "node-e"
    assert pool.get_node("key-b") == "node-d"
    assert pool.get_node("key-c") == "node-b"
    assert pool.get_node("key-d") == "node-a"
    assert pool.get_node("key-e") == "node-e"
    assert pool.get_node("key-aa") == "node-b" # <-

    pool.remove_node("node-e")

    # all keys of "node-e" were re-assigned, others are untouched
    assert pool.get_node("key-a") == "node-c" # <-
    assert pool.get_node("key-b") == "node-d"
    assert pool.get_node("key-c") == "node-b"
    assert pool.get_node("key-d") == "node-a"
    assert pool.get_node("key-e") == "node-b" # <-
    assert pool.get_node("key-aa") == "node-b"
