use std::cmp;
use std::collections::VecDeque;

use generational_arena::{Arena, Index};

#[derive(Debug, Clone)]
pub struct Node<I, V, K> {
    index: Option<I>,
    item: Option<V>,
    child: Option<I>,
    next: Option<I>,
    second_parent: Option<I>,
    key: K,
    rank: usize,
}

impl<T: Ord + Copy> Node<Index, T, T> {
    fn new(item: T) -> Node<Index, T, T> {
        Node {
            index: None,
            item: Some(item),
            child: None,
            next: None,
            second_parent: None,
            key: item,
            rank: 0,
        }
    }

    pub fn new_in_arena(arena: &mut Arena<Node<Index, T, T>>, item: T) -> Index {
        // safe because we assign index later
        let node = Self::new(item);
        let index = arena.insert(node);
        arena[index].index = Some(index);
        index
    }

    pub fn add_child(&mut self, new_child: &mut Node<Index, T, T>) -> Index {
        new_child.next = self.child;
        self.child = Some(new_child.index.unwrap());
        self.index.unwrap()
    }

    fn link(&mut self, other: &mut Self) -> Index {
        // this linking behaviour makes it a max-heap
        if self.key > other.key {
            self.add_child(other)
        } else {
            other.add_child(self)
        }
    }

    fn ranked_link(&mut self, other: &mut Self) -> Index {
        assert!(self.rank == other.rank);
        // this linking behaviour makes it a max-heap
        if self.key > other.key {
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

#[derive(Debug)]
pub struct HollowHeap<T> {
    pub dag: Arena<Node<Index, T, T>>,
    pub dag_root: Option<Index>,
}

impl<T: Ord + Copy> HollowHeap<T> {
    pub fn new() -> HollowHeap<T> {
        HollowHeap {
            dag: Arena::new(),
            dag_root: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.dag.len() == 0
    }

    pub fn push(&mut self, value: T) -> Index {
        let index = Node::new_in_arena(&mut self.dag, value);
        if let Some(root_index) = self.dag_root {
            let (root, node) = self.dag.get2_mut(root_index, index);
            // unwrap should be safe because these indices come from inside the dag
            self.dag_root = Some(root.unwrap().link(node.unwrap()));
        } else {
            self.dag_root = Some(index);
        }
        index
    }

    /// increase the key (used for sorting) of the Node at `index`
    ///
    /// expects `dag_root` to not be empty and `index` to be valid
    pub fn increase_key(&mut self, index: Index, new_val: T) -> Index {
        if self.dag_root == None {
            panic!("Should not be accessing the heap with an invalid index (heap is empty).");
        } else if self.dag_root == Some(index) {
            // the changed value is the root so will be updated in-place
            let ref mut node = self.dag[index];
            node.item = Some(new_val);
            node.key = new_val.into();
            index
        } else {
            // the changed value is not the root and thus will become hollow
            let (new_index, rank) = {
                let node = self
                    .dag
                    .get_mut(index)
                    .expect("Should not be accessing the heap with an invalid index.");
                node.item = None;
                let rank = node.rank;
                (self.push(new_val), rank)
            };
            let second_parent = {
                // we created the new index, so this is fine
                let ref mut new_node = self.dag[new_index];
                new_node.rank = if rank > 1 { rank - 2 } else { 0 };
                if self.dag_root == Some(new_index) {
                    None
                } else {
                    new_node.child = Some(index);
                    Some(new_index)
                }
            };
            // the expect above guarantees that this access is valid
            self.dag[index].second_parent = second_parent;
            new_index
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.dag_root
            .map(|root_index| self.dag[root_index].item.as_ref())
            .unwrap_or(None)
    }

    pub fn delete(&mut self, index: Index) -> Option<Index> {
        if self.dag_root != Some(index) {
            if let Some(node) = self.dag.get_mut(index) {
                node.item = None;
                node.second_parent = None;
                return self.dag_root;
            } else {
                return None;
            }
        }
        // index is the root index from here
        let root_index = index;
        let mut max_rank = 0;
        let mut roots_by_rank = vec![None];
        if let Some(root) = self.dag.get_mut(root_index) {
            root.next = None;
            root.second_parent = None;
        } else {
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
                    if rank >= roots_by_rank.len() {
                        roots_by_rank.resize(rank + 1, None);
                    }
                    while let Some(index) = roots_by_rank[rank] {
                        let (first_node, second_node) = self.dag.get2_mut(index, cur_child_idx);
                        // unwrap should be safe because these indices come from inside the dag
                        cur_child_idx = first_node.unwrap().ranked_link(&mut second_node.unwrap());
                        roots_by_rank[rank] = None;
                        rank = rank + 1;
                        if rank >= roots_by_rank.len() {
                            roots_by_rank.push(None);
                        }
                    }
                    // the ranked_link increased the rank
                    max_rank = cmp::max(rank, max_rank);
                    roots_by_rank.resize(max_rank + 1, None);
                    roots_by_rank[rank] = Some(cur_child_idx);
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
                        next_root = Some(root.unwrap().link(other_root.unwrap()));
                    }
                }
            });
        }
        next_root
    }

    pub fn pop(&mut self) -> Option<T> {
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

#[test]
fn new_heap_is_empty() {
    let heap: HollowHeap<u8> = HollowHeap::new();
    assert!(heap.is_empty());
}

#[test]
fn push_nodes() {
    let mut heap: HollowHeap<u8> = HollowHeap::new();
    assert!(heap.is_empty());
    heap.push(2);
    heap.push(5);
    assert!(!heap.is_empty());
    assert!(heap.dag.len() == 2);
}

#[test]
fn peek_node() {
    let mut heap: HollowHeap<u8> = HollowHeap::new();
    assert!(heap.is_empty());
    heap.push(2);
    heap.push(4);
    assert!(heap.peek() == Some(&4));
}

#[test]
fn pop_node() {
    let mut heap: HollowHeap<u8> = HollowHeap::new();
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
