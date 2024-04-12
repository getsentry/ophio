use indexmap::IndexSet;
use md5::{Digest, Md5};
use smol_str::SmolStr;

/// A node pool that does consistent hashing based on the "Ketama" algorithm.
pub struct KetamaPool {
    /// The set of nodes that this pool knows about.
    nodes: IndexSet<SmolStr>,
    /// The list of Servers, sorted by their ranking value.
    ranking: Vec<NodeRank>,

    /// A reusable scratch buffer to format node hash keys into.
    hash_buf: String,
}

/// A Nodes `rank` in the main [`KetamaPool::ranking`] list.
struct NodeRank {
    /// The hash value, used for sorting.
    value: u32,
    /// The index of the node in the main [`KetamaPool::nodes`] set.
    index: u32,
}

const POINTS_PER_HASH: usize = 4;
const POINTS_PER_SERVER: usize = 40;

impl KetamaPool {
    /// Builds a new pool with the given `initial_nodes`.
    pub fn new(initial_nodes: &[&str]) -> Self {
        let mut slf = Self {
            nodes: initial_nodes.iter().map(SmolStr::new).collect(),
            ranking: vec![],
            hash_buf: String::new(),
        };
        slf.update_node_ranking();
        slf
    }

    /// Adds a new `node` to the pool.
    pub fn add_node(&mut self, node: &str) {
        if self.nodes.insert(node.into()) {
            // in theory its possible to do `add`s incrementally, but its infrequent so probably not worth the effort.
            self.update_node_ranking();
        }
    }

    /// Remove the given `node` from the pool.
    pub fn remove_node(&mut self, node: &str) {
        self.nodes.swap_remove(node);
        self.update_node_ranking();
    }

    /// Returns the node name which will host the given `key`.
    ///
    /// Panics if no node has been added to this pool.
    pub fn get_node(&self, key: &str) -> &str {
        let idx = self.get_node_idx(key);
        self.nodes.get_index(idx).unwrap()
    }

    /// Picks a node in this pool to host the given `key`.
    pub fn get_node_idx(&self, key: &str) -> usize {
        if self.ranking.len() <= 1 {
            return 0;
        }

        let key_hash = if key.is_empty() {
            0
        } else {
            crc32fast::hash(key.as_ref())
        };

        let ranking_idx = match self
            .ranking
            .binary_search_by_key(&key_hash, |rank| rank.value)
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        self.ranking[ranking_idx % self.ranking.len()].index as usize
    }

    fn update_node_ranking(&mut self) {
        self.ranking.clear();
        self.ranking
            .reserve(POINTS_PER_SERVER * POINTS_PER_HASH * self.nodes.len());

        for (idx, key) in self.nodes.iter().enumerate() {
            for point_idx in 0..POINTS_PER_SERVER {
                use std::fmt::Write;
                self.hash_buf.clear();
                write!(&mut self.hash_buf, "{key}-{point_idx}").unwrap();
                let md5_hash = Md5::digest(&self.hash_buf);

                for alignment in 0..POINTS_PER_HASH {
                    let value = u32::from_be_bytes([
                        md5_hash[3 + alignment * 4],
                        md5_hash[2 + alignment * 4],
                        md5_hash[1 + alignment * 4],
                        md5_hash[alignment * 4],
                    ]);
                    self.ranking.push(NodeRank {
                        value,
                        index: idx as u32,
                    });
                }
            }
        }

        self.ranking.sort_by_key(|rank| rank.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hashing() {
        let mut pool = KetamaPool::new(&["node-a", "node-b", "node-c", "node-d", "node-e"]);

        assert_eq!(pool.get_node("key-a"), "node-e");
        assert_eq!(pool.get_node("key-b"), "node-d");
        assert_eq!(pool.get_node("key-c"), "node-b");
        assert_eq!(pool.get_node("key-d"), "node-a");
        assert_eq!(pool.get_node("key-e"), "node-e");
        assert_eq!(pool.get_node("key-aa"), "node-b");

        pool.add_node("node-f");

        // most existing keys are unchanged
        assert_eq!(pool.get_node("key-a"), "node-e");
        assert_eq!(pool.get_node("key-b"), "node-d");
        assert_eq!(pool.get_node("key-c"), "node-b");
        assert_eq!(pool.get_node("key-d"), "node-a");
        assert_eq!(pool.get_node("key-e"), "node-e");
        // one key has moved to the new node
        assert_eq!(pool.get_node("key-aa"), "node-f"); // <-

        pool.remove_node("node-f");

        // we are back to the original assignment
        assert_eq!(pool.get_node("key-a"), "node-e");
        assert_eq!(pool.get_node("key-b"), "node-d");
        assert_eq!(pool.get_node("key-c"), "node-b");
        assert_eq!(pool.get_node("key-d"), "node-a");
        assert_eq!(pool.get_node("key-e"), "node-e");
        assert_eq!(pool.get_node("key-aa"), "node-b"); // <-

        pool.remove_node("node-e");

        // all keys of "node-e" were re-assigned, others are untouched
        assert_eq!(pool.get_node("key-a"), "node-c"); // <-
        assert_eq!(pool.get_node("key-b"), "node-d");
        assert_eq!(pool.get_node("key-c"), "node-b");
        assert_eq!(pool.get_node("key-d"), "node-a");
        assert_eq!(pool.get_node("key-e"), "node-b"); // <-
        assert_eq!(pool.get_node("key-aa"), "node-b");
    }
}
