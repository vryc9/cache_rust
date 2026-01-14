use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;


/// Définit le comportement standard d'un Cache.
///
/// Ce trait permet d'interchanger différentes implémentations de cache
/// (ex: LRU, FIFO, LFU) sans changer le code qui l'utilise.
pub trait Cache<K, V> {
    /// Crée un nouveau cache avec une capacité fixe.
    fn new(capacity: usize) -> Self;

    /// Insère une paire clé-valeur dans le cache.
    /// Si la capacité est atteinte, l'algorithme d'éviction se déclenche.
    fn put(&mut self, key: K, value: V);

    /// Récupère une référence vers la valeur associée à la clé.
    /// Cette action met généralement à jour les métadonnées d'utilisation (ex: récence).
    fn get(&mut self, key: &K) -> Option<&V>;

    /// Retourne le nombre d'éléments actuellement stockés.
    fn len(&self) -> usize;
}

/// Un nœud interne utilisé dans l'Arena (`Vec`).
///
/// Il stocke la donnée réelle ainsi que les indices des voisins
/// pour simuler une liste doublement chaînée.
#[derive(Debug)]
pub(crate) struct Node<K, V> {
    pub(crate) key: K,
    pub(crate) value: V,
    /// Index du nœud précédent (plus récent). `None` si c'est la Tête.
    pub(crate) prev: Option<usize>,
    /// Index du nœud suivant (plus vieux). `None` si c'est la Queue.
    pub(crate) next: Option<usize>,
}

/// Une implémentation d'un Cache LRU (Least Recently Used).
///
/// # Architecture
/// Ce cache utilise une approche "Arena" pour maximiser la performance et la localité du cache CPU :
/// * **HashMap** : Associe `Clé -> Index` (pour un accès O(1)).
/// * **Vec (Arena)** : Stocke les `Node` de manière contiguë.
/// * **Indices** : Utilise des `usize` au lieu de pointeurs pour lier les nœuds.
///
/// 
pub struct LruCache<K, V> {
    /// Capacité maximale du cache.
    capacity: usize,
    /// Annuaire pour trouver l'index d'une clé en O(1).
    pub(crate) map: HashMap<K, usize>,
    /// Stockage physique des nœuds.
    pub(crate) arena: Vec<Node<K, V>>,
    /// Index de l'élément le plus récemment utilisé (Tête de liste).
    pub head: Option<usize>,
    /// Index de l'élément le moins récemment utilisé (Queue de liste).
    pub(crate) tail: Option<usize>,
}

impl<K, V> Cache<K, V> for LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Debug,
{
    /// Crée un nouveau Cache LRU vide.
    ///
    /// # Arguments
    /// * `capacity` - Le nombre maximum d'éléments avant éviction.
    ///
    /// # Panics
    /// Panique si `capacity` est 0.
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "La capacité doit être > 0");
        LruCache {
            capacity,
            map: HashMap::with_capacity(capacity),
            arena: Vec::with_capacity(capacity),
            head: None,
            tail: None,
        }
    }

    /// Récupère une valeur.
    ///
    /// # Effets de bord
    /// Si la clé est trouvée, l'élément est déplacé en **Tête** de liste
    /// (marqué comme le plus récent).
    ///
    /// # Complexité
    /// O(1)
    fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(&index) = self.map.get(key) {
            self.move_to_head(index);
            return Some(&self.arena[index].value);
        }
        None
    }

    /// Insère ou met à jour une valeur.
    ///
    /// * Si la clé existe : met à jour la valeur et déplace en Tête.
    /// * Si la clé n'existe pas :
    ///     * Si plein : supprime le LRU (Tail).
    ///     * Insère le nouvel élément en Tête.
    ///
    /// # Complexité
    /// O(1) amorti (grâce au `swap_remove` sur le vecteur).
    fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // Cas 1: Mise à jour
            let index = self.map[&key];
            self.arena[index].value = value;
            self.move_to_head(index);
        } else {
            // Cas 2: Insertion
            if self.arena.len() >= self.capacity {
                self.remove_lru();
            }

            let index = self.arena.len();
            let node = Node {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            };

            self.arena.push(node);
            self.map.insert(key, index);

            if let Some(old_head_idx) = self.head {
                self.arena[old_head_idx].prev = Some(index);
            }

            self.head = Some(index);

            if self.tail.is_none() {
                self.tail = Some(index);
            }
        }
    }

    fn len(&self) -> usize {
        self.arena.len()
    }
}

// --- Méthodes Internes (Private) ---
impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
{
    /// Déplace un nœud existant vers la position `head`.
    /// Met à jour les liens `prev` et `next` des voisins.
    fn move_to_head(&mut self, index: usize) {
        if Some(index) == self.head {
            return;
        }
        let prev_idx = self.arena[index].prev;
        let next_idx = self.arena[index].next;
        
        // Détachement du nœud
        if let Some(prev) = prev_idx {
            self.arena[prev].next = next_idx;
        }
        if let Some(next) = next_idx {
            self.arena[next].prev = prev_idx;
        }

        // Mise à jour du Tail si nécessaire
        if Some(index) == self.tail {
            self.tail = prev_idx;
        }

        // Insertion en Tête
        if let Some(old_head) = self.head {
            self.arena[old_head].prev = Some(index);
        }

        self.arena[index].next = self.head;
        self.arena[index].prev = None;
        self.head = Some(index);
    }

    /// Supprime l'élément le moins récemment utilisé (Tail).
    ///
    /// # Stratégie d'éviction
    /// Utilise `swap_remove` pour supprimer l'élément du vecteur en O(1).
    /// Cela déplace le dernier élément du vecteur à l'index supprimé.
    /// Il faut donc "patcher" les liens de cet élément déplacé.
    /// 
    /// 
    fn remove_lru(&mut self) {
        if let Some(tail_idx) = self.tail {
            // 1. Suppression logique de la Map
            let key_to_remove = self.arena[tail_idx].key.clone();
            self.map.remove(&key_to_remove);

            // 2. Mise à jour du pointeur Tail
            self.tail = self.arena[tail_idx].prev;
            
            if let Some(new_tail) = self.tail {
                self.arena[new_tail].next = None;
            } else {
                self.head = None;
            }

            // 3. Suppression physique et Patching des indices
            self.arena.swap_remove(tail_idx);

            // Si l'élément supprimé n'était pas le dernier physique du tableau,
            // un autre élément a pris sa place (celui qui était à la fin).
            if tail_idx < self.arena.len() {
                let moved_key = self.arena[tail_idx].key.clone();
                self.map.insert(moved_key, tail_idx);
                
                let prev = self.arena[tail_idx].prev;
                let next = self.arena[tail_idx].next;
  
                if let Some(p) = prev {
                    self.arena[p].next = Some(tail_idx);
                }
                if let Some(n) = next {
                    self.arena[n].prev = Some(tail_idx);
                }

                if self.head == Some(self.arena.len()) {
                    self.head = Some(tail_idx);
                }
                if self.tail == Some(self.arena.len()) {
                    self.tail = Some(tail_idx);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_simple_put_get() {
        let mut cache = LruCache::new(2);
        cache.put("A", 1);
        cache.put("B", 2);
        assert_eq!(cache.get(&"A"), Some(&1));
        assert_eq!(cache.get(&"B"), Some(&2));
    }

    #[test]
    fn test_eviction_lru() {
        let mut cache = LruCache::new(2);
        cache.put("A", 1);
        cache.put("B", 2);
        cache.get(&"A");
        cache.put("C", 3);
        assert_eq!(cache.get(&"B"), None);
        assert_eq!(cache.get(&"A"), Some(&1));
        assert_eq!(cache.get(&"C"), Some(&3));
    }
}
