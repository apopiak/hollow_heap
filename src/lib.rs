use std::cmp;
use std::collections::VecDeque;

type NodeIndex = usize;

#[derive(Debug)]
pub struct Pool<T> {
    pub data: Vec<Option<T>>,
    pub open_indices: Vec<NodeIndex>,
    pub len: usize,
}

impl<T: Sized> Pool<T> {
    pub fn new() -> Pool<T> {
        Pool { data: vec![], open_indices: vec![0], len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn insert(&mut self, item: T) -> NodeIndex {
        self.len += 1;
        match self.open_indices.pop() {
            Some(index) => {
                if index == self.data.len() {
                    self.data.push(Some(item));
                    self.open_indices.push(self.data.len());
                } else {
                    self.data[index] = Some(item);
                }
                index
            },
            None => panic!("should not be empty!"),
        }
    }

    pub fn remove(&mut self, index: NodeIndex) -> Option<T> {
        self.len -= 1;
        self.data.push(None);
        let item = self.data.swap_remove(index);
        self.open_indices.push(index);
        item
    }

    pub fn next_index(& self) -> NodeIndex {
        match self.open_indices.last() {
            Some(next) => next.clone(),
            None => panic!("The open_indices should not be empty!"),
        }
    }

    pub fn get(&self, index: NodeIndex) -> &T {
        unsafe {
            match self.data.get_unchecked(index) {
                &Some(ref item) => item,
                &None => panic!("There should be no invalid indices into the Pool!"),
            }
        }
    }

    pub fn get_mut(&mut self, index: NodeIndex) -> &mut T {
        unsafe {
            match self.data.get_unchecked_mut(index) {
                &mut Some(ref mut item) => item,
                &mut None => panic!("There should be no invalid indices into the Pool!"),
            }
        }
    }

    pub fn split_at_mut(&mut self, mid: NodeIndex) -> (&mut [Option<T>], &mut [Option<T>]) {
        self.data.split_at_mut(mid)
    }

    pub fn borrow_two_mut(&mut self, first: NodeIndex, second: NodeIndex) -> (&mut T, &mut T) {
        let expectation = "The two indices should be valid.";
        if first > second {
            let (mut first_slice, mut second_slice) = self.split_at_mut(first);
            unsafe {
                (second_slice.get_unchecked_mut(0).as_mut().expect(expectation),
                first_slice.get_unchecked_mut(second).as_mut().expect(expectation))
            }
        } else if second > first {
            let (mut first_slice, mut second_slice) = self.split_at_mut(second);
            unsafe {
                (first_slice.get_unchecked_mut(first).as_mut().expect(expectation),
                second_slice.get_unchecked_mut(0).as_mut().expect(expectation))
            }
        } else {
            panic!("The two indices should not point to the same address!");
        }
    }
}

#[test]
fn empty_pool_has_open_slot() {
    let pool: Pool<i32> = Pool::new();
    assert!(pool.data.len() == 0);
    assert!(pool.open_indices.len() == 1);
}

#[test]
fn insert_into_empty_pool() {
    let mut pool = Pool::new();
    pool.insert(5);
    pool.insert(2);
    assert!(pool.data.len() == 2);
    assert!(pool.open_indices.len() == 1);
}

#[test]
fn remove_from_pool() {
    let mut pool = Pool::new();
    pool.insert(5);
    let index = pool.insert(2);
    pool.insert(3);
    assert!(pool.data.len() == 3);
    assert!(pool.open_indices.len() == 1);
    let item = pool.remove(index);
    assert!(item == Some(2));
    assert!(pool.data.len() == 3);
    assert!(pool.open_indices.len() == 2);
}

#[derive(Debug, Clone)]
pub struct Node<T, K> {
    index: NodeIndex,
    item: Option<T>,
    child: Option<NodeIndex>,
    next: Option<NodeIndex>,
    second_parent: Option<NodeIndex>,
    key: K,
    rank: usize,
}

impl<T: Copy, K: Ord + Copy> Node<T, K> {
    pub fn new(index: NodeIndex, item: T, key: K) -> Node<T, K> {
        Node {
            index: index,
            item: Some(item),
            child: None,
            next: None,
            second_parent: None,
            key: key,
            rank: 0,
        }
    }

    pub fn add_child(&mut self, new_child: &mut Node<T, K>) -> NodeIndex {
        new_child.next = self.child;
        self.child = Some(new_child.index);
        self.index
    }

    fn link(&mut self, other: &mut Self) -> NodeIndex {
        // this linking behaviour makes it a max-heap
        if self.key > other.key {
            self.add_child(other)
        } else {
            other.add_child(self)
        }
    }

    fn ranked_link(&mut self, other: &mut Self) -> NodeIndex {
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
    // data: Vec<T>,
    pub dag: Pool<Node<T, T>>,
    pub dag_root: Option<NodeIndex>,
}

impl<T: Ord + Copy> HollowHeap<T> {
    pub fn new () -> HollowHeap<T> {
        HollowHeap { dag: Pool::new(), dag_root: None }
    }

    pub fn is_empty(&self) -> bool {
        self.dag.len() == 0
    }

    pub fn push(&mut self, value: T) -> NodeIndex {
        let mut node = Node::new(self.dag.next_index(), value, value);
        match self.dag_root {
            None            => self.dag_root = Some(node.index),
            Some(dag_idx)   => self.dag_root = Some(self.dag.get_mut(dag_idx).link(&mut node)),
        }
        self.dag.insert(node)
    }

    pub fn increase_key(&mut self, index: NodeIndex, new_val: T) -> NodeIndex {
        if self.dag_root == None {
            panic!("Should not be accessing the heap with an invalid index (heap is empty).");
        } else if self.dag_root == Some(index) { // the changed value is the root so will be updated in-place
            let ref mut node = self.dag.get_mut(index);
            node.item = Some(new_val);
            node.key = new_val.into();
            index
        } else { // the changed value is not the root and thus will become hollow
            let new_index = self.push(new_val);
            let rank = {
                let ref mut node = self.dag.get_mut(index);
                node.item = None;
                node.rank
            };
            let second_parent = {
                let ref mut new_node = self.dag.get_mut(new_index);
                new_node.rank = if rank > 1 { rank - 2 } else { 0 };
                if self.dag_root == Some(new_index) {
                    None
                } else {
                    new_node.child = Some(index);
                    Some(new_index)
                }
            };
            self.dag.get_mut(index).second_parent = second_parent;
            new_index
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.dag_root.map(|root_index| {
            self.dag.get(root_index).item.as_ref()
        }).unwrap_or(None)
    }

    pub fn delete(&mut self, index: NodeIndex) -> Option<NodeIndex> {
        if self.dag_root != Some(index) {
            let mut node = self.dag.get_mut(index);
            node.item = None;
            node.second_parent = None;
            return self.dag_root;
        }
        // index is the root index from here
        let mut max_rank = 0;
        let mut roots_by_rank = vec![None];
        self.dag.get_mut(index).next = None;
        self.dag.get_mut(index).second_parent = None;
        let mut queue = VecDeque::new();
        queue.push_back(index);
        let mut next_root = None;
        while let Some(current_root) = queue.pop_front() {
            let to_delete = current_root;
            let mut next_child = {
                let ref root = self.dag.get_mut(current_root);
                next_root = root.next;
                root.child
            };
            while let Some(child_idx) = next_child {
                next_child = self.dag.get(child_idx).next;
                if self.dag.get_mut(child_idx).is_hollow() {
                    let mut current_child = self.dag.get_mut(child_idx);
                    match current_child.second_parent {
                        None => {
                            current_child.next = next_root;
                            next_root = Some(current_child.index);
                        },
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
                    let mut rank = self.dag.get(cur_child_idx).rank;
                    if rank >= roots_by_rank.len() {
                        roots_by_rank.resize(rank + 1, None);
                    }
                    while let Some(index) = roots_by_rank[rank] {
                        let (mut first_node, mut second_node) = self.dag.borrow_two_mut(index, cur_child_idx);
                        cur_child_idx = first_node.ranked_link(&mut second_node);
                        roots_by_rank[rank] = None;
                        rank = rank + 1;
                        if rank >= roots_by_rank.len() {
                            roots_by_rank.push(None);
                        }
                    }
                    max_rank = cmp::max(rank, max_rank); // the ranked_link increased the rank
                    roots_by_rank.resize(max_rank + 1, None);
                    roots_by_rank[rank] = Some(cur_child_idx);
                }
            }
            next_root.map(|next_index| { queue.push_back(next_index) });
            self.dag.remove(to_delete);
        }
        for root in roots_by_rank {
            root.map(|root_index| {
                match next_root {
                    None => next_root = Some(root_index),
                    Some(next_root_index) => {
                        let (ref mut root, ref mut other_root) = self.dag.borrow_two_mut(next_root_index, root_index);
                        next_root = Some(root.link(other_root));
                    },
                }
            });
        }
        next_root
    }

    pub fn pop(&mut self) -> Option<T> {
        let (result, new_root_idx) = self.dag_root.map(|root_index| {
            let item = self.dag.get_mut(root_index).item.take();
            (item, self.delete(root_index))
        }).unwrap_or((None, None));
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
