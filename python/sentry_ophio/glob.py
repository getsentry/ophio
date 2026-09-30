from ._bindings import is_glob_match as _is_glob_match

__all__ = ["is_glob_match"]


def is_glob_match(
    value: str | bytes,
    pat: str | bytes,
    double_star: bool = False,
    case_insensitive: bool = False,
    path_normalize: bool = False,
    allow_newline: bool = False,
) -> bool:
    """Match a glob against text or bytes.

    With ``double_star``, only ``**`` matches across path separators. ``path_normalize``
    replaces backslashes with slashes in both inputs. ``allow_newline`` lets wildcards
    match newlines. Invalid patterns return ``False``.
    """
    if case_insensitive:
        value = value.lower()
        pat = pat.lower()

    if isinstance(value, str):
        value = value.encode("utf-8")
    if isinstance(pat, bytes):
        pat = pat.decode("utf-8")

    return _is_glob_match(value, pat, double_star, case_insensitive, path_normalize, allow_newline)
