class KetamaPool:
    """
    A Consistent hashing pool based on the "Ketama" algorithm.
    """
    def __new__(cls, slots: list[str]) -> KetamaPool:
        """
        Creates a new consistent hashing pool, using the given `slots` as keys.
        """

    def get_slot(
        self, key: str
    ) -> int:
        """
        Returns the index within the initially provided `slots` to which the
        given `key` is being associated.
        """

    def add_node(self, key: str):
        """
        Add a new node
        """

    def remove_node(self, key: str):
        """
        Remove a node
        """

    def get_node(self, key: str) -> str:
        """
        Get a node
        """
