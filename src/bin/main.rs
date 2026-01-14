use cache_lru_project::{LruCache, Cache};

fn main() {
    let filename = "cache.txt";
    let mut cache: LruCache<String, String> = LruCache::new_persistent(3, filename)
        .expect("Erreur création cache");

    let data_sequence = vec!["A", "B", "C", "D", "B", "A", "E"];
    for key in data_sequence {
        if let Some(val) = cache.get(&key.to_string()) {
            println!("{}", val);
        } else {
            let value = format!("{}", key);
            cache.put(key.to_string(), value);
        }
    }
    cache.save_to_file(filename).expect("Erreur sauvegarde");
}