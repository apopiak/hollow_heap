#[macro_use]
extern crate proptest;
use proptest::prelude::*;

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
    fn doesnt_crash_with_delete_and_increase_key(vector in vec(u32::arbitrary(), 2..1000)) {
        println!("{:?}", vector);
        let mut heap = HollowHeap::max_heap();
        let mut index = None;
        let mut second_index = None;
        for num in vector.iter() {
            if index.is_none() {
                index = Some(heap.push(*num));
            } else if second_index.is_none() {
                second_index = Some(heap.push(*num));
            }
        }

        let index = index.unwrap();
        let second_index = second_index.unwrap();

        let value = *heap.peek().unwrap();
        heap.increase_key(index, value + 1);
        heap.delete(second_index);
    }
}
