use md5::{Digest, Md5};

pub struct KetamaPool {
    /// The list of Servers, sorted by their ranking value.
    ranking: Vec<ServerRank>,
}

struct ServerRank {
    value: u32,
    index: u32,
}

const POINTS_PER_HASH: usize = 4;
const POINTS_PER_SERVER: usize = 40;

impl KetamaPool {
    /// Builds a new pool using the given hash keys.
    pub fn new(keys: &[&str]) -> Self {
        let mut slf = Self { ranking: vec![] };
        slf.update_node_ranking(keys);
        slf
    }

    /// Picks a slot for the given `key`.
    ///
    /// The "slot" here is an index into the origin list of keys this pool was constructed with.
    pub fn get_slot(&self, key: &str) -> usize {
        if self.ranking.len() == 1 {
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

    fn update_node_ranking(&mut self, keys: &[&str]) {
        self.ranking.clear();
        self.ranking
            .reserve(POINTS_PER_SERVER * POINTS_PER_HASH * keys.len());

        let mut hash_buf = String::new();
        for (idx, key) in keys.iter().enumerate() {
            for point_idx in 0..POINTS_PER_SERVER {
                use std::fmt::Write;
                hash_buf.clear();
                write!(&mut hash_buf, "{key}-{point_idx}").unwrap();
                let md5_hash = Md5::digest(&hash_buf);

                for alignment in 0..POINTS_PER_HASH {
                    let value = u32::from_be_bytes([
                        md5_hash[3 + alignment * 4],
                        md5_hash[2 + alignment * 4],
                        md5_hash[1 + alignment * 4],
                        md5_hash[alignment * 4],
                    ]);
                    self.ranking.push(ServerRank {
                        value,
                        index: idx as u32,
                    });
                }
            }
        }

        self.ranking.sort_by_key(|rank| rank.value);
    }
}
