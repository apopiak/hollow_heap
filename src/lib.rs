/*!
A heap with great asymptotic run-time based on the
[hollow heap paper](https://arxiv.org/abs/1510.06535).

**Note: pre-alpha software unfit for production.**

Extra Note: The hollow heap (at least in this implementation) has high overhead per node because of
all the book-keeping that needs to be done.

## Why implement it then?

Fun! Also:
All heap operations in a hollow heap except `delete` and `pop` take O(1) time.

## Features

* Zero `unsafe` (by using `generational_arena`)

## Usage

First, add `hollow_heap` to your `Cargo.toml`:

```toml
[dependencies]
hollow_heap = "0.5"
```

Then, import the crate and use the
[`hollow_heap::HollowHeap`](./struct.HollowHeap.html) type!

```rust
use hollow_heap::HollowHeap;

let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();

// Insert some elements into the heap.
heap.push(3);
heap.push(8);
heap.push(17);
heap.push(5);
heap.push(9);

// And we will get the elements back in sorted order when `pop`ing.
println!("{:?}", heap.pop()); // 17
println!("{:?}", heap.pop()); // 9
println!("{:?}", heap.pop()); // 8
println!("{:?}", heap.pop()); // 5
println!("{:?}", heap.pop()); // 3
println!("{:?}", heap.pop()); // None
```

Feel free to use the [`hollow_heap::HollowHeapBuilder`](./struct.HollowHeapBuilder.html) type to
configure your `HollowHeap` in a flexible way. A contrived example:
```rust
use hollow_heap::{HollowHeap, HollowHeapBuilder};

let mut heap: HollowHeap<f32, u16> = HollowHeapBuilder::new(|uint:&u16| f32::from(*uint) + 0.5)
    .with_compare(|lhs, rhs| lhs < rhs)
    .with_capacity(100)
    .build();
heap.push(42);
heap.push(21);
heap.push(1);

println!("{:?}", heap.pop()); // 1
println!("{:?}", heap.pop()); // 21
println!("{:?}", heap.pop()); // 42
println!("{:?}", heap.pop()); // None
```
 */
use std::cmp;
use std::collections::VecDeque;

use generational_arena::{Arena, Index};

/// A node in a hollow heap.
///
/// `item` holds the value.
/// `key` is used for comparison.
/// Hollow nodes are represented by setting `item` to `None`.
#[derive(Debug, Clone)]
struct Node<I, K, V> {
    index: Option<I>,
    item: Option<V>,
    child: Option<I>,
    next: Option<I>,
    second_parent: Option<I>,
    key: K,
    /// Quote from the [hollow heap paper](https://arxiv.org/abs/1510.06535):
    /// > The rank of a node in a [...] hollow heap of `N` nodes is at most `logφN` (logarithm of N
    /// with the golden ratio)
    ///
    /// This means that the rank in a hollow heap whose size is limited by `usize` is never greater
    /// than `logφ(usize::max_value) = 92.18688578640361` and thus fits in a `u8`.
    rank: u8,
}

impl<K: PartialOrd, V> Node<Index, K, V> {
    /// Note: incomplete because index is not set correctly.
    fn new(item: V, key: K) -> Node<Index, K, V> {
        Node {
            index: None,
            item: Some(item),
            child: None,
            next: None,
            second_parent: None,
            key,
            rank: 0,
        }
    }

    pub fn new_in_arena(arena: &mut Arena<Node<Index, K, V>>, item: V, key: K) -> Index {
        // safe because we assign index later
        let node = Self::new(item, key);
        let index = arena.insert(node);
        arena[index].index = Some(index);
        index
    }

    fn add_child(&mut self, new_child: &mut Node<Index, K, V>) -> Index {
        new_child.next = self.child;
        self.child = Some(new_child.index.unwrap());
        self.index.unwrap()
    }

    fn link(&mut self, other: &mut Self, compare: fn(lhs: &K, rhs: &K) -> bool) -> Index {
        if compare(&self.key, &other.key) {
            self.add_child(other)
        } else {
            other.add_child(self)
        }
    }

    fn ranked_link(&mut self, other: &mut Self, compare: fn(lhs: &K, rhs: &K) -> bool) -> Index {
        assert!(self.rank == other.rank);
        if compare(&self.key, &other.key) {
            self.rank += 1;
            self.add_child(other)
        } else {
            other.rank += 1;
            other.add_child(self)
        }
    }

    pub fn is_hollow(&self) -> bool {
        self.item.is_none()
    }
}

/// The comparison function used for the min heap.
pub fn min_heap_compare<K: PartialOrd>(lhs: &K, rhs: &K) -> bool {
    lhs < rhs
}

/// The comparison function used for the max heap.
pub fn max_heap_compare<K: PartialOrd>(lhs: &K, rhs: &K) -> bool {
    lhs > rhs
}

/// The `HollowHeap` allows inserting elements into and removing elements from a heap, returning
/// the items in the order implied by the chosen compare function. Can be used, for example, as a
/// priority queue.
///
/// [See the module-level documentation for example usage and motivation.](./index.html)
#[derive(Clone)]
pub struct HollowHeap<K, V> {
    dag: Arena<Node<Index, K, V>>,
    dag_root: Option<Index>,
    pub compare: fn(&K, &K) -> bool,
    pub derive_key: fn(&V) -> K,
}

use std::fmt;
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for HollowHeap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HollowHeap {{ dag_root: {:?}, dag: {:?} }}",
            self.dag_root, self.dag
        )
    }
}

impl<K: PartialOrd + fmt::Debug, V> HollowHeap<K, V> {
    pub fn new(compare: fn(&K, &K) -> bool, derive_key: fn(&V) -> K) -> HollowHeap<K, V> {
        HollowHeap {
            dag: Arena::new(),
            dag_root: None,
            compare,
            derive_key,
        }
    }

    /// Test whether there are any elements in the heap.
    pub fn is_empty(&self) -> bool {
        self.dag.len() == 0
    }

    /// Push a value into the heap.
    ///
    /// Returns the index of the pushed element.
    pub fn push(&mut self, value: V) -> Index {
        let key = (self.derive_key)(&value);
        self.push_with_key(value, key)
    }

    /// Push a value into the heap with the provided key.
    ///
    /// Circumvents the `derive_key` function and is thus not recommended.
    pub fn push_with_key(&mut self, value: V, key: K) -> Index {
        let index = Node::new_in_arena(&mut self.dag, value, key);
        if let Some(root_index) = self.dag_root {
            let (root, node) = self.dag.get2_mut(root_index, index);
            // unwrap should be safe because these indices come from inside the dag
            self.dag_root = Some(root.unwrap().link(node.unwrap(), self.compare));
        } else {
            self.dag_root = Some(index);
        }
        index
    }

    /// Increase or decrease the key (used for sorting) of the Node at `index`.
    ///
    /// **Note:** This function only changes the key, not the item.
    ///
    /// Expects (and asserts) `dag_root` to not be empty and `index` to be valid.
    /// Asserts that `new_key` is greater (or smaller) than the old key (depending on the type
    /// of heap).
    pub fn change_key(&mut self, index: Index, new_key: K) -> Index {
        self.update(index, None, new_key.into())
    }

    /// Change the item (and recalculate the key) of the Node at `index`.
    ///
    /// Expects (and asserts) `dag_root` to not be empty and `index` to be valid.
    /// Asserts that the new generated key is greater (or smaller) than the old key (depending on
    /// the type of heap).
    pub fn change_item(&mut self, index: Index, new_item: V) -> Index {
        self.update(index, new_item.into(), None)
    }

    fn update(&mut self, index: Index, new_item: Option<V>, new_key: Option<K>) -> Index {
        assert_ne!(
            self.dag_root, None,
            "Should not be trying to change key on empty heap."
        );
        let ref item_ref = &new_item;
        let new_key = new_key.unwrap_or_else(|| {
            (self.derive_key)(
                item_ref
                    .as_ref()
                    .expect("Need either a new item or a new key to update."),
            )
        });
        if self.dag_root == Some(index) {
            // the changed value is the root so will be updated in-place
            let ref mut node = self.dag[index];
            assert!(
                (self.compare)(&new_key, &node.key),
                format!("Should only increase key to 'better' value. '{:?}' is not 'better' than '{:?}'", new_key, node.key)
            );
            if let Some(item) = new_item {
                node.item = Some(item);
            }
            node.key = new_key.into();
            return index;
        }
        // the changed value is not the root and thus will become hollow
        // let (new_index, rank) = {
        let node = self
            .dag
            .get_mut(index)
            .expect("Should not be accessing the heap with an invalid index.");
        assert!(
            (self.compare)(&new_key, &node.key),
            format!(
                "Should only increase key to 'better' value. '{:?}' is not 'better' than '{:?}'",
                new_key, node.key
            )
        );
        let item = {
            let old_item = node
                .item
                .take()
                .expect("Should not be changing the key of an item twice.");
            if let Some(item) = new_item {
                item
            } else {
                old_item
            }
        };
        let rank = node.rank;

        // };
        let new_index = self.push_with_key(item, new_key);
        let second_parent = {
            // we created a node and got the new index, so this access is fine
            let ref mut new_node = self.dag[new_index];
            new_node.rank = if rank > 1 { rank - 2 } else { 0 };
            if self.dag_root == Some(new_index) {
                None
            } else {
                new_node.child = Some(index);
                Some(new_index)
            }
        };
        // `index` is assumed valid; the expect above guarantees that this is the case
        self.dag[index].second_parent = second_parent;
        new_index
    }

    /// Have a look at the top-most value of the heap.
    ///
    /// Returns `None` if the heap is empty.
    pub fn peek(&self) -> Option<&V> {
        self.dag_root
            .map(|root_index| self.dag[root_index].item.as_ref())
            .unwrap_or(None)
    }

    /// Remove the value at `index` from the heap.
    ///
    /// Returns the new root index if successful and `None` if deletion failed or the heap is empty
    /// after the operation.
    pub fn delete(&mut self, index: Index) -> Option<Index> {
        if self.dag_root != Some(index) {
            if let Some(node) = self.dag.get_mut(index) {
                node.item = None;
                node.second_parent = None;
                return self.dag_root;
            }
            // nothing todo if item is not present in dag
            // println!("No element found to delete at {:?}", index);
            return None;
        }
        // index is the root index from here
        let root_index = index;
        let mut max_rank = 0;
        let mut roots_by_rank = vec![None];
        if let Some(root) = self.dag.get_mut(root_index) {
            root.next = None;
            root.second_parent = None;
        } else {
            // println!("No root found to delete at {:?}", root_index);
            return None;
        }
        let mut queue = VecDeque::new();
        queue.push_back(root_index);
        let mut next_root = None;
        while let Some(current_root) = queue.pop_front() {
            let to_delete = current_root;
            let mut next_child = {
                let ref root = self.dag[current_root];
                next_root = root.next;
                root.child
            };
            while let Some(child_idx) = next_child {
                next_child = self.dag[child_idx].next;
                if self.dag[child_idx].is_hollow() {
                    let ref mut current_child = self.dag[child_idx];
                    match current_child.second_parent {
                        None => {
                            current_child.next = next_root;
                            next_root = Some(current_child.index.unwrap());
                        }
                        Some(_) => {
                            if current_child.second_parent == Some(to_delete) {
                                next_child = None;
                            } else {
                                current_child.next = None;
                            }
                            current_child.second_parent = None;
                        }
                    }
                } else {
                    let mut cur_child_idx = child_idx;
                    let mut rank = self.dag[cur_child_idx].rank;
                    if rank as usize >= roots_by_rank.len() {
                        roots_by_rank.resize((rank + 1) as usize, None);
                    }
                    while let Some(index) = roots_by_rank[rank as usize] {
                        let (first_node, second_node) = self.dag.get2_mut(index, cur_child_idx);
                        // unwrap should be safe because these indices come from inside the dag
                        cur_child_idx = first_node
                            .unwrap()
                            .ranked_link(&mut second_node.unwrap(), self.compare);
                        roots_by_rank[rank as usize] = None;
                        rank = rank + 1;
                        if rank as usize >= roots_by_rank.len() {
                            roots_by_rank.push(None);
                        }
                    }
                    // the ranked_link increased the rank
                    max_rank = cmp::max(rank, max_rank);
                    roots_by_rank.resize((max_rank + 1) as usize, None);
                    roots_by_rank[rank as usize] = Some(cur_child_idx);
                }
            }
            next_root.map(|next_index| queue.push_back(next_index));
            self.dag.remove(to_delete);
        }
        for root_idx in roots_by_rank {
            root_idx.map(|root_index| {
                match next_root {
                    None => next_root = Some(root_index),
                    Some(next_root_index) => {
                        let (root, other_root) = self.dag.get2_mut(next_root_index, root_index);
                        // unwrap should be safe because these indices come from inside the dag
                        next_root = Some(root.unwrap().link(other_root.unwrap(), self.compare));
                    }
                }
            });
        }
        self.dag_root = next_root;
        // return the index of the next root
        next_root
    }

    /// Remove the top-most value from the heap and return it.
    ///
    /// Returns `None` if the heap is empty.
    pub fn pop(&mut self) -> Option<V> {
        let (result, new_root_idx) = self
            .dag_root
            .map(|root_index| {
                let item = self.dag[root_index].item.take();
                (item, self.delete(root_index))
            })
            .unwrap_or((None, None));
        self.dag_root = new_root_idx;
        result
    }
}

impl<T: PartialOrd + Copy> HollowHeap<T, T> {
    /// Create a new heap with the specified capacity. Defaults to a min heap.
    ///
    /// The heap will be able to hold `n` elements without further allocation.
    pub fn with_capacity(n: usize) -> HollowHeap<T, T> {
        HollowHeap {
            dag: Arena::with_capacity(n),
            dag_root: None,
            compare: min_heap_compare,
            derive_key: |value| *value,
        }
    }

    /// Create a new empty heap with the chosen compare function.
    pub fn with_compare(compare: fn(&T, &T) -> bool) -> HollowHeap<T, T> {
        HollowHeap {
            dag: Arena::new(),
            dag_root: None,
            compare,
            derive_key: |value| *value,
        }
    }

    /// Create a new empty heap with the chosen compare function and the specified capacity.
    ///
    /// The heap will be able to hold `n` elements without further allocation.
    pub fn with_compare_and_capacity(compare: fn(&T, &T) -> bool, n: usize) -> HollowHeap<T, T> {
        HollowHeap {
            dag: Arena::with_capacity(n),
            dag_root: None,
            compare,
            derive_key: |value| *value,
        }
    }

    /// Create a new max heap. (`compare = |lhs, rhs| lhs > rhs`)
    pub fn max_heap() -> HollowHeap<T, T> {
        HollowHeap::with_compare(max_heap_compare)
    }

    /// Create a new min heap. (`compare = |lhs, rhs| lhs < rhs`)
    pub fn min_heap() -> HollowHeap<T, T> {
        HollowHeap::with_compare(min_heap_compare)
    }
}

#[cfg(test)]
mod tests {
    use super::HollowHeap;

    #[test]
    fn new_heap_is_empty() {
        let heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
    }

    #[test]
    fn push_nodes() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(5);
        assert!(!heap.is_empty());
        assert!(heap.dag.len() == 2);
    }

    #[test]
    fn peek_node() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(4);
        assert!(heap.peek() == Some(&4));
    }

    #[test]
    fn pop_node_max_heap() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(8);
        heap.push(4);
        heap.push(9);
        heap.push(1);
        assert!(heap.pop() == Some(9));
        assert!(heap.pop() == Some(8));
        assert!(heap.pop() == Some(4));
        assert!(heap.pop() == Some(2));
        assert!(heap.pop() == Some(1));
        assert!(heap.pop() == None);
    }

    #[test]
    fn pop_node_min_heap() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::min_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(8);
        heap.push(4);
        heap.push(9);
        heap.push(1);
        assert!(heap.pop() == Some(1));
        assert!(heap.pop() == Some(2));
        assert!(heap.pop() == Some(4));
        assert!(heap.pop() == Some(8));
        assert!(heap.pop() == Some(9));
        assert!(heap.pop() == None);
    }

    #[test]
    fn change_key_with_min_heap() {
        let mut heap: HollowHeap<u16, u16> = HollowHeap::min_heap();
        assert!(heap.is_empty());
        heap.push(5);
        let index = heap.push(42);
        heap.push(4);
        heap.change_key(index, 2);
        assert!(heap.pop() == Some(42));
        assert!(heap.pop() == Some(4));
        assert!(heap.pop() == Some(5));
        assert!(heap.pop() == None);
    }

    #[test]
    fn change_item_with_min_heap() {
        let mut heap: HollowHeap<u16, u16> = HollowHeap::min_heap();
        assert!(heap.is_empty());
        heap.push(5);
        let index = heap.push(42);
        heap.push(4);
        heap.change_item(index, 2);
        assert!(heap.pop() == Some(2));
        assert!(heap.pop() == Some(4));
        assert!(heap.pop() == Some(5));
        assert!(heap.pop() == None);
    }

    #[test]
    #[should_panic]
    fn faulty_change_key_panics() {
        let mut heap: HollowHeap<u16, u16> = HollowHeap::min_heap();
        assert!(heap.is_empty());
        heap.push(5);
        let index = heap.push(1);
        heap.push(4);
        heap.change_key(index, 2);
    }

    #[test]
    fn push_same_values() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(2);
        heap.push(1);
        assert!(!heap.is_empty());
        assert!(heap.dag.len() == 3);
        assert!(heap.pop() == Some(2));
        assert!(heap.pop() == Some(2));
        assert!(heap.pop() == Some(1));
        assert!(heap.pop() == None);
    }

    #[derive(PartialEq, Eq)]
    struct SomeStruct {
        some_value: u32,
    }

    #[test]
    fn different_key_from_value() {
        let mut heap: HollowHeap<u32, &SomeStruct> =
            HollowHeap::new(|lhs, rhs| lhs > rhs, |val| val.some_value);
        assert!(heap.is_empty());
        let first = SomeStruct { some_value: 2 };
        heap.push(&first);
        let second = SomeStruct { some_value: 3 };
        heap.push(&second);
        let third = SomeStruct { some_value: 1 };
        heap.push(&third);
        assert!(!heap.is_empty());
        assert!(heap.dag.len() == 3);
        assert!(heap.pop() == Some(&second));
        assert!(heap.pop() == Some(&first));
        assert!(heap.pop() == Some(&third));
        assert!(heap.pop() == None);
    }

    #[test]
    fn change_item_with_complex_value() {
        let mut heap: HollowHeap<u32, &SomeStruct> =
            HollowHeap::new(|lhs, rhs| lhs < rhs, |val| val.some_value);
        assert!(heap.is_empty());
        let first = SomeStruct { some_value: 42 };
        let index = heap.push(&first);
        let second = SomeStruct { some_value: 3 };
        heap.push(&second);
        let third = SomeStruct { some_value: 1 };
        heap.push(&third);
        let changed = SomeStruct { some_value: 2 };
        heap.change_item(index, &changed);
        assert!(!heap.is_empty());
        assert!(heap.pop() == Some(&third));
        assert!(heap.pop() == Some(&changed));
        assert!(heap.pop() == Some(&second));
        assert!(heap.pop() == None);
    }

}

/// A builder to construct a [`HollowHeap`](./struct.HollowHeap.html).
/// Allows specifying key derivation and compare functions as well as capacity.
#[derive(Clone)]
pub struct HollowHeapBuilder<K, V> {
    capacity: Option<usize>,
    compare: fn(&K, &K) -> bool,
    derive_key: fn(&V) -> K,
}

impl<K: PartialOrd, V> HollowHeapBuilder<K, V> {
    /// Create a new HollowHeapBuilder to configure and build a HollowHeap.
    ///
    /// Every HollowHeap needs a `derive_key` function. Consider `|val| *val` for trivial values
    /// (like `u32` or `f64`).
    pub fn new(derive_key: fn(&V) -> K) -> HollowHeapBuilder<K, V> {
        HollowHeapBuilder {
            capacity: None,
            compare: min_heap_compare,
            derive_key,
        }
    }

    /// Specify the capacity of the heap. The heap will not allocate for the first `n` elements
    /// pushed into it.
    pub fn with_capacity(&mut self, n: usize) -> &mut HollowHeapBuilder<K, V> {
        self.capacity = Some(n);
        self
    }

    /// Specify the compare function to use.
    pub fn with_compare(&mut self, compare: fn(&K, &K) -> bool) -> &mut HollowHeapBuilder<K, V> {
        self.compare = compare;
        self
    }

    /// Set the compare function in the way to get a min heap.
    pub fn min_heap(&mut self) -> &mut HollowHeapBuilder<K, V> {
        self.compare = min_heap_compare;
        self
    }

    /// Set the compare function in the way to get a max heap.
    pub fn max_heap(&mut self) -> &mut HollowHeapBuilder<K, V> {
        self.compare = max_heap_compare;
        self
    }

    /// Finish constructing the HollowHeap and return it.
    pub fn build(&self) -> HollowHeap<K, V> {
        if let Some(capacity) = self.capacity {
            HollowHeap {
                dag: Arena::with_capacity(capacity),
                dag_root: None,
                compare: self.compare,
                derive_key: self.derive_key,
            }
        } else {
            HollowHeap {
                dag: Arena::new(),
                dag_root: None,
                compare: self.compare,
                derive_key: self.derive_key,
            }
        }
    }
}

impl<T: PartialOrd + Copy> HollowHeapBuilder<T, T> {
    pub fn new_with_value_is_key() -> HollowHeapBuilder<T, T> {
        HollowHeapBuilder {
            capacity: None,
            compare: min_heap_compare,
            derive_key: |value| *value,
        }
    }
}

#[cfg(test)]
mod builder_tests {
    use super::HollowHeapBuilder;

    #[derive(PartialEq, Eq)]
    struct St {
        val: u32,
    }

    #[test]
    fn builder_builds_heap() {
        let capacity = 5;
        let mut builder = HollowHeapBuilder::new(|st: &St| st.val);
        let mut heap = builder
            .with_compare(|lhs, rhs| lhs < rhs)
            .with_capacity(capacity)
            .build();
        assert!(heap.dag.capacity() == capacity);
        heap.push(St { val: 50 });
        heap.push(St { val: 40 });
        heap.push(St { val: 30 });

        assert!(heap.pop() == Some(St { val: 30 }));
        assert!(heap.pop() == Some(St { val: 40 }));
        assert!(heap.pop() == Some(St { val: 50 }));
        assert!(heap.pop() == None);
    }
}

impl<K: PartialOrd + fmt::Debug, V> IntoIterator for HollowHeap<K, V> {
    type Item = V;
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { inner: self }
    }
}

#[derive(Clone, Debug)]
pub struct IntoIter<K, V> {
    inner: HollowHeap<K, V>,
}

impl<K: PartialOrd + fmt::Debug, V> Iterator for IntoIter<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.dag.len();
        (len, Some(len))
    }
}

#[cfg(test)]
mod iter_tests {
    use super::HollowHeap;

    #[test]
    fn iterator_returns_sorted_items() {
        let mut heap: HollowHeap<u8, u8> = HollowHeap::max_heap();
        assert!(heap.is_empty());
        heap.push(2);
        heap.push(8);
        heap.push(4);
        heap.push(9);
        heap.push(1);
        let mut iter = heap.into_iter();
        assert!(iter.next() == Some(9));
        assert!(iter.next() == Some(8));
        assert!(iter.next() == Some(4));
        assert!(iter.next() == Some(2));
        assert!(iter.next() == Some(1));
        assert!(iter.next() == None);
    }
}
