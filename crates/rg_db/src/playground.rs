use std::marker::PhantomData;

use crate::{NodeType, PAGE_SIZE};

struct Split<K, V> {
    new_node: Box<dyn Node<K, V>>,
    new_key: K,
}

trait Node<K, V> {
    fn get_type(&self) -> NodeType;
    fn insert(
        &mut self,
        key: K,
        value: V,
        owner: &mut Tree<K, V>,
    ) -> Result<Option<Split<K, V>>, String>;

    fn search(&self, key: &K) -> Option<&V>;

    fn remove(&mut self, key: &K) -> Result<bool, String>;
}

struct InnerNode<K, V> {
    keys: Vec<K>,
    children: Vec<Box<dyn Node<K, V>>>,
}

impl<K, V> InnerNode<K, V>
where
    K: Copy + Ord,
{
    const KEY_SIZE: usize = std::mem::size_of::<K>();
    const VALUE_SIZE: usize = std::mem::size_of::<Box<dyn Node<K, V>>>();
    const MAX_PAIRS: usize = const { PAGE_SIZE / (Self::KEY_SIZE + Self::VALUE_SIZE) };
    const MAX_KEYS: usize = Self::MAX_PAIRS.saturating_sub(1);

    fn new() -> Self {
        Self {
            keys: Vec::default(),
            children: Vec::default(),
        }
    }

    fn split(&mut self) -> (K, InnerNode<K, V>) {
        let mid = self.keys.len() / 2;
        let new_key = self.keys[mid];
        let mut new_node = InnerNode::<K, V>::new();
        let mut right_half = self.keys.split_off(mid + 1);
        new_node.keys.append(&mut right_half);
        self.keys.pop(); // pop new key
        let mut right_half = self.children.split_off(mid + 1);
        new_node.children.append(&mut right_half);

        (new_key, new_node)
    }

    ///
    /// Returns two indices: first is where key should be put, second which child to use
    ///
    fn binary_search(&self, key: &K) -> (usize, usize) {
        match self.keys.binary_search(&key) {
            Ok(idx) => {
                if *key >= self.keys[idx] {
                    (idx + 1, idx + 1)
                } else {
                    (idx, idx)
                }
            }
            Err(idx) => {
                if idx == self.keys.len() {
                    (idx, idx)
                } else if *key >= self.keys[idx] {
                    (idx + 1, idx + 1)
                } else {
                    (idx, idx)
                }
            }
        }
    }
}

impl<K, V> Node<K, V> for InnerNode<K, V>
where
    K: Copy + Ord + 'static,
    V: 'static,
{
    fn get_type(&self) -> NodeType {
        NodeType::Inner
    }

    fn insert(
        &mut self,
        key: K,
        value: V,
        owner: &mut Tree<K, V>,
    ) -> Result<Option<Split<K, V>>, String> {
        let (key_idx, node_idx) = self.binary_search(&key);

        let result = self.children[node_idx].insert(key, value, owner)?;
        if let Some(split) = result {
            self.keys.insert(key_idx, split.new_key);
            self.children.insert(node_idx + 1, split.new_node);

            if self.keys.len() == Self::MAX_KEYS {
                let (new_key, new_node) = self.split();
                return Ok(Some(Split {
                    new_node: Box::new(new_node),
                    new_key: new_key,
                }));
            }
        }

        Ok(None)
    }

    fn search(&self, key: &K) -> Option<&V> {
        let (_, node_idx) = self.binary_search(&key);
        self.children[node_idx].search(key)
    }

    fn remove(&mut self, key: &K) -> Result<bool, String> {
        let (_, node_idx) = self.binary_search(&key);
        self.children[node_idx].remove(key)
    }
}

struct LeafNode<K, V> {
    keys: Vec<K>,
    data: Vec<V>,
}

impl<K, V> LeafNode<K, V>
where
    K: Copy,
{
    const KEY_SIZE: usize = std::mem::size_of::<K>();
    const VALUE_SIZE: usize = std::mem::size_of::<V>();
    const MAX_PAIRS: usize = const { PAGE_SIZE / (Self::KEY_SIZE + Self::VALUE_SIZE) };
    const MAX_KEYS: usize = Self::MAX_PAIRS;

    fn new() -> Self {
        Self {
            keys: Vec::default(),
            data: Vec::default(),
        }
    }

    fn split(&mut self) -> (K, LeafNode<K, V>) {
        let mid = self.keys.len() / 2;
        let new_key = self.keys[mid];
        let mut new_node = LeafNode::<K, V>::new();
        let mut right_half = self.keys.split_off(mid);
        new_node.keys.append(&mut right_half);
        let mut right_half = self.data.split_off(mid);
        new_node.data.append(&mut right_half);

        (new_key, new_node)
    }
}

impl<K, V> Node<K, V> for LeafNode<K, V>
where
    K: Copy + Ord + 'static,
    V: 'static,
{
    fn get_type(&self) -> NodeType {
        NodeType::Leaf
    }

    fn insert(
        &mut self,
        key: K,
        value: V,
        owner: &mut Tree<K, V>,
    ) -> Result<Option<Split<K, V>>, String> {
        match self.keys.binary_search(&key) {
            Ok(idx) => {
                self.data[idx] = value;
            }
            Err(idx) => {
                self.keys.insert(idx, key);
                self.data.insert(idx, value);

                if self.keys.len() == Self::MAX_KEYS {
                    let (new_key, new_node) = self.split();
                    return Ok(Some(Split {
                        new_key,
                        new_node: Box::new(new_node),
                    }));
                }
            }
        }

        Ok(None)
    }

    fn search(&self, key: &K) -> Option<&V> {
        let idx = self.keys.binary_search(key).ok()?;
        Some(&self.data[idx])
    }

    fn remove(&mut self, key: &K) -> Result<bool, String> {
        if let Ok(idx) = self.keys.binary_search(key) {
            self.keys.remove(idx);
            self.data.remove(idx);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

struct Tree<K, V> {
    root: Option<Box<dyn Node<K, V>>>,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Tree<K, V>
where
    K: Copy + Ord + 'static,
    V: 'static,
{
    fn new() -> Self {
        Self {
            root: None,
            _phantom: PhantomData::default(),
        }
    }

    fn ensure_root(&mut self) -> Result<(), String> {
        if self.root.is_none() {
            self.root = Some(Box::new(LeafNode::new()));
            if self.root.is_none() {
                return Err("Failed to allocate node!".to_string());
            }
        }
        Ok(())
    }

    fn insert(&mut self, key: K, value: V) -> Result<(), String> {
        let _ = self.ensure_root()?;

        if let Some(mut root) = self.root.take() {
            if let Some(split) = root.insert(key, value, self)? {
                let mut new_root = Box::new(InnerNode::<K, V>::new());

                new_root.keys.push(split.new_key);
                new_root.children.push(root);
                new_root.children.push(split.new_node);

                self.root = Some(new_root);
            } else {
                self.root = Some(root);
            }
        }

        Ok(())
    }

    fn search(&self, key: &K) -> Option<&V> {
        self.root.as_ref().and_then(|root| root.search(key))
    }

    fn remove(&mut self, key: &K) -> Result<bool, String> {
        self.root
            .as_mut()
            .ok_or("Not found".to_string())
            .and_then(|root| root.remove(key))
    }
}

#[cfg(test)]
mod tests {
    use crate::playground::Tree;

    #[test]
    fn should_insert() {
        let mut tree = Tree::<i32, u64>::new();

        for i in 0..256_000 {
            tree.insert(i, i as u64 * 10).unwrap();
        }

        for i in 0..256_000 {
            let result = tree.search(&i).unwrap();
            assert_eq!(i as u64 * 10, *result);
        }

        for i in 0..256_000 {
            let result = tree.remove(&i).unwrap();
            assert!(result);
        }

        for i in 0..256_000 {
            let result = tree.search(&i);
            assert!(result.is_none());
        }
    }
}
