from typing import Any, Mapping, Optional, Sequence, Union

import pytest
from sentry_ophio.enhancers import Cache, Component, Enhancements

# TODO: all this is copied from Sentry, and the Sentry side should still
# be responsible for the `create_match_frame`
PathSearchable = Union[Mapping[str, Any], Sequence[Any], None]

cache = Cache(1_000)

def get_path(data: PathSearchable, *path, **kwargs):
    """
    Safely resolves data from a recursive data structure. A value is only
    returned if the full path exists, otherwise ``None`` is returned.

    If the ``default`` argument is specified, it is returned instead of ``None``.

    If the ``filter`` argument is specified and the value is a list, it is
    filtered with the given callback. Alternatively, pass ``True`` as filter to
    only filter ``None`` values.
    """
    default = kwargs.pop("default", None)
    f: Optional[bool] = kwargs.pop("filter", None)
    for k in kwargs:
        raise TypeError("get_path() got an undefined keyword argument '%s'" % k)

    for p in path:
        if isinstance(data, Mapping) and p in data:
            data = data[p]
        elif isinstance(data, (list, tuple)) and isinstance(p, int) and -len(data) <= p < len(data):
            data = data[p]
        else:
            return default

    if f and data and isinstance(data, (list, tuple)):
        data = list(filter((lambda x: x is not None) if f is True else f, data))

    return data if data is not None else default


def create_match_frame(frame_data: dict, platform: Optional[str]) -> dict:
    """Create flat dict of values relevant to matchers"""
    match_frame = dict(
        category=get_path(frame_data, "data", "category"),
        family=frame_data.get("platform") or platform,
        function=frame_data.get("function"),
        in_app=frame_data.get("in_app") or False,
        orig_in_app=get_path(frame_data, "data", "orig_in_app"),
        module=get_path(frame_data, "module"),
        package=frame_data.get("package"),
        path=frame_data.get("abs_path") or frame_data.get("filename"),
    )

    for key in list(match_frame.keys()):
        value = match_frame[key]
        if isinstance(value, (bytes, str)):
            if key in ("package", "path"):
                value = match_frame[key] = value.lower()

            if isinstance(value, str):
                match_frame[key] = value.encode("utf-8")

    return match_frame


def test_simple_enhancer():
    enhancer = Enhancements.parse("path:**/test.js              +app", cache)

    frames = [
        create_match_frame(
            {"abs_path": "http://example.com/foo/test.js", "filename": "/foo/test.js"},
            "javascript",
        )
    ]
    exception_data = {"ty": None, "value": None, "mechanism": None}

    modified_frames = enhancer.apply_modifications_to_frames(frames, exception_data)
    print(modified_frames)


@pytest.mark.parametrize("action", ["+", "-"])
@pytest.mark.parametrize("type", ["prefix", "sentinel"])
def test_sentinel_and_prefix(action, type):
    enhancer = Enhancements.parse(f"function:foo {action}{type}", cache)

    frames = [create_match_frame({"function": "foo"}, "whatever")]
    frame_components = [Component(contributes=None, is_prefix_frame=False, is_sentinel_frame=False)]

    assert not getattr(frame_components[0], f"is_{type}_frame")

    exception_data = {"ty": None, "value": None, "mechanism": None}
    enhancer.assemble_stacktrace_component(frames, exception_data,frame_components)

    expected = action == "+"
    assert getattr(frame_components[0], f"is_{type}_frame") is expected


def test_parsing_errors():
    with pytest.raises(RuntimeError, match="failed to parse matchers"):
        Enhancements.parse("invalid.message:foo -> bar", cache)


def test_caller_recursion():
    # Remove this test when CallerMatch can be applied recursively
    with pytest.raises(RuntimeError, match="failed to parse matchers"):
        Enhancements.parse("[ category:foo ] | [ category:bar ] | category:baz +app", cache)


def test_callee_recursion():
    # Remove this test when CalleeMatch can be applied recursively
    with pytest.raises(RuntimeError, match="failed to parse actions"):
        Enhancements.parse(" category:foo | [ category:bar ] | [ category:baz ] +app", cache)
