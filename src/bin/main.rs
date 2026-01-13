use cache_lru_project::{LruCache, Cache};

fn main() {
    let filename = "cache.txt";
    let mut cache: LruCache<String, String> = LruCache::new_persistent(3, filename)
        .expect("Erreur création cache");

    let data_sequence = vec!["A", "B", "C", "D", "B", "A", "E"];
    for key in data_sequence {
        if let Some(val) = cache.get(&key.to_string()) {
            println!("HIT ! Trouvé en cache : {}", val);
        } else {
            let value = format!("value_of_{}", key);
            cache.put(key.to_string(), value);
        }
    }

    println!("\n--- État Final ---");
    println!("Contenu de 'B' (devrait être là) : {:?}", cache.get(&"B".to_string()));
    println!("Contenu de 'A' (devrait être évincé par E si taille 3) : {:?}", cache.get(&"A".to_string()));

    cache.save_to_file(filename).expect("Erreur sauvegarde");
    println!("enregistré dans {}'", filename);
}