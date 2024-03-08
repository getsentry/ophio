from dataclasses import dataclass
from uuid import UUID

@dataclass(frozen=True)
class JavaStackFrame:
    class_name: str
    method: str
    file: str | None
    line: int

class ProguardMapper:
    @staticmethod
    def open(path: str) -> ProguardMapper: ...
    @property
    def uuid(self) -> UUID: ...
    @property
    def has_line_info(self) -> bool: ...
    def remap_method(self, klass: str, method: str) -> tuple[str, str] | None: ...
    def remap_frame(
        self, klass: str, method: str, line: int, parameters: str | None = None
    ) -> list[JavaStackFrame]: ...
