from typing import Any

Frame = dict[str, Any]
ModificationResult = tuple[str | None, bool | None]

class Component:
    contributes: bool | None
    is_prefix_frame: bool
    is_sentinel_frame: bool
    hint: str | None

class AssembleResult:
    contributes: bool
    hint: str | None
    invert_stacktrace: bool

class Cache:
    """
    An LRU cache for memoizing the construction of regexes and enhancement rules.

    :param size: The number of both rules and regexes that will be cached.
    """

    def __new__(cls, size: int) -> Cache: ...

class Enhancements:
    """
    A suite of enhancement rules.
    """

    @staticmethod
    def empty() -> Enhancements:
        """
        Creates an Enhancements object with no rules.
        """
    @staticmethod
    def parse(input: str, cache: Cache) -> Enhancements:
        """
        Parses an Enhancements object from a string.

        :param input: The input string.
        :param cache: A cache that memoizes rule and regex construction.
        """
    @staticmethod
    def from_config_structure(input: bytes, cache: Cache) -> Enhancements:
        """
        Parses an Enhancements object from the msgpack representation.

        :param input: The input in msgpack format.
        :param cache: A cache that memoizes rule and regex construction.
        """
    def extend_from(self, other: Enhancements):
        """
        Adds all rules from the other Enhancements object to this one.
        """
    def apply_modifications_to_frames(
        self,
        frames: list[Frame],
        exception_data: dict[str, str | None],
    ) -> list[ModificationResult]:
        """
        Modifies a list of frames according to the rules in this Enhancements object.

        The returned list contains the new values of the "category" and
        "in_app" fields for each frame.

        :param frames: The list of frames to modify.
        :param exception_data: Exception data to match against rules. Supported
                               fields are "ty", "value", and "mechanism".
        """
    def assemble_stacktrace_component(
        self,
        frames: list[Frame],
        exception_data: dict[str, str | None],
        components: list[Component],
    ) -> AssembleResult:
        """
        Modifies a list of `Component`s according to the rules in this Enhancements object.

        It returns an `AssembleResult` containing attributes of a resulting
        `stacktrace` grouping component,
        which has to be assembled outside of this function.

        :param frames: The list of frames to analyze.
        :param exception_data: Exception data to match against rules. Supported
                               fields are "ty", "value", and "mechanism".
        :param components: The list of `Component`s to modify.
                           The `Component` objects are mutated in place.
        """
