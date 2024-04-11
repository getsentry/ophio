use md5::{Digest, Md5};

pub struct KetamaPool {
    /// The list of Servers, sorted by their ranking value.
    pub ranking: Vec<ServerRank>,
}

pub struct ServerRank {
    pub server_name: String,
    pub value: u32,
    pub index: u32,
}

impl KetamaPool {
    /// Builds a new pool using the given hash keys.
    pub fn new(keys: &[&str]) -> Self {
        let mut ranking = Vec::with_capacity(POINTS_PER_SERVER * POINTS_PER_HASH * keys.len());
        create_server_ranking(keys, &mut ranking);
        Self { ranking }
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

    /// Adds the appropriate rankings for the new node
    pub fn add_node(&mut self, server_name: &str) {
        create_server_ranking(&[server_name], &mut self.ranking);
    }

    /// Removes the rankings corresponding to the server name
    pub fn remove_node(&mut self, server_name: &str) {
        self.ranking
            .retain(|ranking| ranking.server_name == server_name)
    }

    pub fn get_node(&self, key: &str) -> &str {
        let slot = self.get_slot(key);
        self.ranking[slot].server_name.as_ref()
    }
}

const POINTS_PER_HASH: usize = 4;
const POINTS_PER_SERVER: usize = 40;

fn create_server_ranking(keys: &[&str], ranking: &mut Vec<ServerRank>) {
    let mut hash_buf = String::new();

    for (idx, key) in keys.iter().enumerate() {
        for point_idx in 0..POINTS_PER_SERVER {
            use std::fmt::Write;
            hash_buf.clear();
            write!(&mut hash_buf, "{key}-{point_idx}").unwrap();
            let md5_hash = Md5::digest(&hash_buf);

            for alignment in 0..POINTS_PER_HASH {
                let value = ketama_hash(md5_hash.as_slice(), alignment);
                ranking.push(ServerRank {
                    server_name: key.to_string(),
                    value,
                    index: idx as u32,
                });
            }
        }
    }

    ranking.sort_by_key(|rank| rank.value);
}

fn ketama_hash(md5_hash: &[u8], alignment: usize) -> u32 {
    u32::from_be_bytes([
        md5_hash[3 + alignment * 4],
        md5_hash[2 + alignment * 4],
        md5_hash[1 + alignment * 4],
        md5_hash[alignment * 4],
    ])
}

#[cfg(test)]
mod tests {
    use crate::ketama::*;
    use md5::{Digest, Md5};

    #[test]
    fn it_produces_correct_ketama_hashes() {
        let pool = KetamaPool::new(&["server1"]);
        assert_eq!(pool.ranking.len(), 1 * POINTS_PER_SERVER * POINTS_PER_HASH);

        let hash = Md5::digest("server1-8");
        /*
        Results taken from twemproxy tests:

        expect_same_uint32_t(3853726576U, ketama_hash("server1-8", strlen("server1-8"), 0), "should have expected ketama_hash for server1-8 index 0");
        expect_same_uint32_t(2667054752U, ketama_hash("server1-8", strlen("server1-8"), 3), "should have expected ketama_hash for server1-8 index 3");
        */
        assert_eq!(3853726576, ketama_hash(hash.as_slice(), 0));
        assert_eq!(2667054752, ketama_hash(hash.as_slice(), 3));
    }

    #[test]
    fn it_can_add_and_remove_servers() {
        let mut pool = KetamaPool::new(&["server1"]);
        pool.add_node("server2");
        assert_eq!(2 * POINTS_PER_SERVER * POINTS_PER_HASH, pool.ranking.len());

        pool.remove_node("server2");
        assert_eq!(1 * POINTS_PER_SERVER * POINTS_PER_HASH, pool.ranking.len());
    }

    #[test]
    fn it_can_get_nodes() {
        let pool = KetamaPool::new(&["server1", "server2"]);
        let result = pool.get_node("organization:2");
        assert_eq!("server2", result);

        let result = pool.get_node("organization:1");
        assert_eq!("server1", result);
    }
}
