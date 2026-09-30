import pytest
from sentry_ophio.codeowners import is_codeowners_path_match


@pytest.mark.parametrize("value_as_bytes", [False, True])
@pytest.mark.parametrize("pattern_as_bytes", [False, True])
@pytest.mark.parametrize(
    "value,pattern,expected",
    [
        ("file.txt", "*.txt", True),
        ("file.txt/", "*.txt", True),
        ("dir/file.txt", "*.txt", True),
        ("file.TXT", "*.txt", False),
        ("/dir/file.txt", "/dir/*.txt", True),
        ("dir/file.txt", "/dir/*.txt", True),
        ("/dir/subdir/file.txt", "/dir/*.txt", False),
        ("apps/file.txt", "apps/", True),
        ("/apps/file.txt", "apps/", True),
        ("/dir/apps/file.txt", "apps/", True),
        ("/dir/subdir/apps/file.txt", "apps/", True),
        ("apps", "apps/", False),
        ("apps/", "apps/", True),
        ("docs/getting-started.md", "docs/*", True),
        ("docs/build-app/troubleshooting.md", "docs/*", False),
        ("something/docs/build-app/troubleshooting.md", "docs/*", False),
        ("first/docs/troubleshooting.md", "*/docs/", True),
        ("docs/getting-started.md", "*/docs/", False),
        ("first/second/docs/troubleshooting.md", "*/docs/", False),
        ("docs/first/something/troubleshooting.md", "docs/*/something/", True),
        ("something/docs/first/something/troubleshooting.md", "docs/*/something/", False),
        ("docs/first/second/something/getting-started.md", "docs/*/something/", False),
        ("/docs/file.txt", "/docs/", True),
        ("/docs/subdir/file.txt", "/docs/", True),
        ("docs/subdir/file.txt", "/docs/", True),
        ("app/docs/file.txt", "/docs/", False),
        ("red/orange/yellow/green/file.py", "/red/**/file.py", True),
        ("/red/orange/file.py", "/red/**/file.py", True),
        ("red/file.py", "/red/**/file.py", True),
        ("yellow/file.py", "/red/**/file.py", False),
        ("red/orange/yellow/green/file.py", "red/**/file.py", True),
        ("red/file.py", "red/**/file.py", True),
        ("something/red/file.py", "red/**/file.py", False),
        ("red/orange/yellow/file.py", "**/yellow/file.py", True),
        ("yellow/file.py", "**/yellow/file.py", True),
        ("/docs/file.py", "**/yellow/file.py", False),
        ("red/orange/yellow/file.py", "/red/orange/**", True),
        ("red/orange/file.py", "/red/orange/**", True),
        ("blue/red/orange/file.py", "/red/orange/**", False),
        ("/docs/subdir/file.css", "/**/*.css", True),
        ("file.css", "/**/*.css", True),
        ("/docs/file.txt", "/**/*.css", False),
        ("foo.py", "/**/foo.py", True),
        ("dir/foo.py", "/**/foo.py", True),
        ("dir/subdir/foo.py", "/**/foo.py", True),
        ("not_foo.py", "/**/foo.py", False),
        ("dir/not_foo.py", "/**/foo.py", False),
        ("file.py", "file.?y", True),
        ("file/py", "file?py", False),
        ("file.py/child", "file.py", True),
        ("file.py.bak", "file.py", False),
        ("[file].py", "[file].py", True),
        ("f.py", "[file].py", False),
        ("!file.py", "!file.py", True),
        ("file.py", "!file.py", False),
        ("\\", "\\", True),
        ("\\/file.py", "\\", True),
        ("file.py", "\\", False),
        ("foo/\\", "\\filename", True),
        ("foo/subdir/\\filename", "\\filename", False),
        ("foo/subdir/\\/backslash_dir", "\\filename", True),
        ("config/subdir/test.py", "\\filename", False),
        ("/usr/local/src/foo/", "/", True),
        ("/usr/local/src/config/subdir/test.py", "/", True),
        ("café.py", "café.py", True),
        ("café/sub/file.py", "café/**", True),
        ("café/sub\n/file.py", "café/**", False),
        ("文档/sub/file.py", "文档/**", True),
        ("文档/sub\n/file.py", "文档/**", False),
        ("", "*", True),
        ("", "", True),
    ],
)
def test_codeowners_path_match(
    value: str,
    pattern: str,
    expected: bool,
    value_as_bytes: bool,
    pattern_as_bytes: bool,
) -> None:
    value_input: str | bytes = value.encode("utf-8") if value_as_bytes else value
    pattern_input: str | bytes = pattern.encode("utf-8") if pattern_as_bytes else pattern
    assert is_codeowners_path_match(value_input, pattern_input) is expected


def test_binary_value() -> None:
    assert is_codeowners_path_match(b"\xff/file.py", "*.py")


def test_keyword_arguments() -> None:
    assert is_codeowners_path_match(value="docs/file.py", pattern="docs/*")
