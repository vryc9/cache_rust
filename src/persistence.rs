use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::str::FromStr;
use std::fmt::{Debug, Display};
use std::hash::Hash;

// On importe les définitions depuis le module parent
use crate::cache::{LruCache, Cache};

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug + Display + FromStr,
    V: Debug + Display + FromStr,
    <K as FromStr>::Err: Debug,
    <V as FromStr>::Err: Debug,
{
    pub fn new_persistent(capacity: usize, filepath: &str) -> io::Result<Self> {
        let mut cache = LruCache::new(capacity);

        if let Ok(file) = File::open(filepath) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(content) = line {
                    if let Some((k_str, v_str)) = content.split_once('=') {
                        let k = K::from_str(k_str).expect("Erreur parsing clé");
                        let v = V::from_str(v_str).expect("Erreur parsing valeur");
                        cache.put(k, v);
                    }
                }
            }
        }
        Ok(cache)
    }

    pub fn save_to_file(&self, filepath: &str) -> io::Result<()> {
        let mut file = File::create(filepath)?;
        
        let mut current_idx = self.tail;
        while let Some(idx) = current_idx {
            let node = &self.arena[idx];
            writeln!(file, "{}={}", node.key, node.value)?;
            current_idx = node.prev; // On remonte vers le plus récent
        }
        
        Ok(())
    }
}