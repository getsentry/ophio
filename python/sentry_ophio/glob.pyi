def is_glob_match(
    value: str | bytes,
    pat: str | bytes,
    double_star: bool = False,
    case_insensitive: bool = False,
    path_normalize: bool = False,
    allow_newline: bool = False,
) -> bool:
    """Match a glob against text or bytes.

    With ``double_star``, only ``**`` matches across path separators.
    ``case_insensitive`` lowercases both inputs before matching, using Unicode
    lowercasing for text and ASCII lowercasing for bytes. ``path_normalize``
    replaces backslashes with slashes in both inputs. ``allow_newline`` lets
    wildcards match newlines. Invalid patterns return ``False``.

    Text values are encoded as UTF-8. Byte patterns must contain valid UTF-8.
    """
