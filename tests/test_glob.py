import pytest
from sentry_ophio.glob import is_glob_match


@pytest.mark.parametrize("value_as_bytes", [False, True])
@pytest.mark.parametrize("pattern_as_bytes", [False, True])
@pytest.mark.parametrize(
    "value,pattern,options,expected",
    [
        ("hello.py", "*.py", {}, True),
        ("hello.py", "*.js", {}, False),
        ("foo/hello.py", "*.py", {}, True),
        ("foo/hello.py", "*.py", {"double_star": True}, False),
        ("foo/hello.py", "**/*.py", {"double_star": True}, True),
        ("hello.py", "**/*.py", {"double_star": True}, True),
        ("foo/hello.PY", "**/*.py", {}, False),
        ("foo/hello.PY", "**/*.py", {"double_star": True}, False),
        ("foo/hello.PY", "**/*.py", {"case_insensitive": True}, True),
        (
            "foo/hello.PY",
            "**/*.py",
            {"double_star": True, "case_insensitive": True},
            True,
        ),
        ("foo\\hello\\bar.PY", "foo/**/*.py", {"case_insensitive": True}, False),
        (
            "foo\\hello\\bar.PY",
            "foo/**/*.py",
            {"double_star": True, "case_insensitive": True},
            False,
        ),
        (
            "foo\\hello\\bar.PY",
            "foo/**/*.py",
            {"double_star": True, "case_insensitive": True, "path_normalize": True},
            True,
        ),
        (
            "foo\\hello\\bar.PY",
            "foo\\**\\*.py",
            {"double_star": True, "case_insensitive": True, "path_normalize": True},
            True,
        ),
        ("foo/bar.py", "foo\\*.py", {"path_normalize": True}, True),
        ("foo\nbar", "foo*", {}, False),
        ("foo\nbar", "foo*", {"allow_newline": True}, True),
        ("1.18.4.2153-2aa83397b", "1.18.[0-4].*", {}, True),
        ("1.18.5.2153-2aa83397b", "1.18.[0-4].*", {}, False),
        ("hello.py", "hello.?y", {}, True),
        ("hello.py", "*.{py,js}", {}, True),
        ("hello.py", "[", {}, False),
        ("hello.py", "{", {}, False),
        ("", "*", {}, True),
        ("", "", {}, True),
        ("hello.py", "", {}, False),
        ("café.py", "*.py", {}, True),
        ("hello\0.py", "*.py", {}, True),
    ],
)
def test_glob_match(
    value: str,
    pattern: str,
    options: dict[str, bool],
    expected: bool,
    value_as_bytes: bool,
    pattern_as_bytes: bool,
) -> None:
    value_input: str | bytes = value.encode("utf-8") if value_as_bytes else value
    pattern_input: str | bytes = pattern.encode("utf-8") if pattern_as_bytes else pattern
    assert is_glob_match(value_input, pattern_input, **options) is expected


@pytest.mark.parametrize(
    "value,pattern,expected",
    [
        ("CAFÉ.py", "café.py", True),
        ("café.py", "CAFÉ.py", True),
        ("CAFÉ.py".encode(), "café.py", False),
        ("café.py", "CAFÉ.py".encode(), False),
        ("CAFÉ.py".encode(), "CAFÉ.py".encode(), True),
        ("HELLO.py".encode(), "hello.py".encode(), True),
    ],
)
def test_case_insensitive_unicode(value: str | bytes, pattern: str | bytes, expected: bool) -> None:
    assert is_glob_match(value, pattern, case_insensitive=True) is expected


def test_binary_value() -> None:
    assert is_glob_match(b"\xff.py", "*.py")


def test_positional_options() -> None:
    assert is_glob_match("foo\\hello\nbar.PY", "foo/**/*.py", True, True, True, True)


def test_keyword_arguments() -> None:
    assert is_glob_match(value="hello.py", pat="*.py")


def test_long_value() -> None:
    value = "x" * 1_000_000 + ".PY"
    options = {"double_star": True, "case_insensitive": True, "path_normalize": True}
    assert is_glob_match(value, "*************************.py", **options)
    assert not is_glob_match(value, "*************************.js", **options)
