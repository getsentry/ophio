def is_codeowners_path_match(value: str | bytes, pattern: str | bytes) -> bool:
    """Match a case-sensitive CODEOWNERS pattern against a path.

    Patterns containing a slash other than a trailing slash are anchored to the
    repository root. A trailing slash matches directories, while a trailing
    ``/*`` only matches immediate children. ``**`` matches across path segments.

    Text values are encoded as UTF-8. Byte patterns must contain valid UTF-8.
    """
