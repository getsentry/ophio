class KetamaPool:
    """
    A Consistent hashing pool based on the "Ketama" algorithm.
    """
    def __new__(cls, nodes: list[str]) -> KetamaPool:
        """
        Creates a new consistent hashing pool, using the given `nodes` as keys.
        """

    def add_node(self, node: str):
        """
        Adds a new `node` to the pool.
        """

    def remove_node(self, node: str):
        """
        Remove the given `node` from the pool.
        """

    def get_node(
        self, key: str
    ) -> str:
        """
        Returns the node name which will host the given `key`.
        """
