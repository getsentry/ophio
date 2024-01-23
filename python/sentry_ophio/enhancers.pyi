from typing import Iterator, Any, List

Frame = dict[str, Any]


class Cache:
    def __new__(cls, size: int) -> 'Cache':
        ...


class Enhancements:
    @staticmethod
    def empty() -> 'Enhancements':
        ...

    @staticmethod
    def parse(input: str, cache: Cache) -> 'Enhancements':
        ...

    @staticmethod
    def from_config_structure(input: bytes, cache: Cache) -> 'Enhancements':
        ...

    def extend_from(self, other: 'Enhancements'):
        ...

    def apply_modifications_to_frames(
        self,
        frames: Iterator[Frame],
        exception_data: dict[str, Any],
    ) -> List[Frame]:
        ...
