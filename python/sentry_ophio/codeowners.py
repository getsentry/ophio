from ._bindings import is_codeowners_path_match as _is_codeowners_path_match

__all__ = ["is_codeowners_path_match"]


def is_codeowners_path_match(value: str | bytes, pattern: str | bytes) -> bool:
    """Match a case-sensitive CODEOWNERS pattern against a path.

    Text values are encoded as UTF-8. Byte patterns must contain valid UTF-8.
    """
    if isinstance(value, str):
        value = value.encode("utf-8")
    if isinstance(pattern, bytes):
        pattern = pattern.decode("utf-8")

    return _is_codeowners_path_match(value, pattern)
