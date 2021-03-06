#[macro_use]
extern crate proptest;
use proptest::prelude::*;

use std::collections::HashMap;

use proptest::collection::vec;

use hollow_heap::HollowHeap;

proptest! {

    #[test]
    fn doesnt_crash(num in 0..100000) {
        let mut heap = HollowHeap::max_heap();
        heap.push(num);
        assert!(heap.pop() == Some(num));
        assert!(heap.pop() == None);
    }

    #[test]
    fn repeated_pop_returns_sorted_vec(vector in vec(u32::arbitrary(), 0..1000)) {
        println!("{:?}", vector);
        let mut heap = HollowHeap::max_heap();
        for num in vector.iter() {
            heap.push(num);
        }

        let mut sorted = vector.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        for num in sorted.iter() {
            prop_assert_eq!(heap.pop(), Some(num));
        }
    }

    #[test]
    fn doesnt_crash_with_delete_and_change_key(vector in vec(u32::arbitrary(), 2..1000)) {
        println!("{:?}", vector);
        let mut heap = HollowHeap::max_heap();
        let mut index = None;
        let mut second_index = None;
        for num in vector.iter() {
            if index.is_none() {
                index = Some(heap.push(*num));
            } else if second_index.is_none() {
                second_index = Some(heap.push(*num));
            } else {
                heap.push(*num);
            }
        }

        let index = index.unwrap();
        let second_index = second_index.unwrap();

        let value = *heap.peek().unwrap();
        heap.change_key(index, value + 1);
        heap.delete(second_index);
        while heap.pop() != None {}
    }

    #[test]
    fn doesnt_crash_with_repeated_delete_and_change_key(vector in vec(u32::arbitrary(), 2..1000)) {
        println!("{:?}", vector);
        let mut heap = HollowHeap::max_heap();
        let mut index_values = HashMap::new();
        for num in vector.iter() {
            let val = *num;
            let idx = heap.push(*num);
            index_values.insert(idx, val);
        }
        for (idx, val) in index_values.iter() {
            if *val < 100 {
                heap.change_key(*idx, val * 2 + 1);
            } else {
                heap.delete(*idx);
            }
        }
        while heap.pop() != None {}
    }

    #[test]
    fn doesnt_crash_with_repeated_operations(vector in vec(u32::arbitrary(), 2..1000)) {
        println!("{:?}", vector);
        let mut heap: HollowHeap<u32, u32> =
            HollowHeap::new(|lhs, rhs| lhs > rhs, |val| *val);
        let mut index_values = HashMap::new();
        for num in vector.iter() {
            let val = *num;
            let idx = heap.push(*num);
            index_values.insert(idx, val);
        }
        for (idx, val) in index_values.iter() {
            if *val < 100 {
                heap.change_key(*idx, val * 2 + 1);
            } else if *val < 300 {
                heap.change_item(*idx, val * 3 + 2);
            } else {
                heap.delete(*idx);
            }
        }
        while heap.pop() != None {}
    }
}
