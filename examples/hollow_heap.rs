extern crate hollow_heap;

use hollow_heap::HollowHeap;

fn example_one() {
    println!(
        "example_one (heap returns the pushed elements in sorted order and then returns None)"
    );
    let mut heap: HollowHeap<u8> = HollowHeap::new();
    heap.push(3);
    heap.push(8);
    heap.push(17);
    heap.push(5);
    heap.push(9);
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    println!("");
}

fn example_vec() {
    println!("example_vec (order of return of the heap equals a sorted vec)");
    let mut my_vec = vec![1, -5, 6, 10, -7, 9, 100000, -555, 666, 100];
    let mut heap = HollowHeap::new();
    {
        for num in my_vec.iter() {
            heap.push(num.clone());
        }
    }
    {
        my_vec.sort_by(|a, b| b.cmp(a)); // it's a max heap so we have to do a 'reverse' sort
    }
    {
        for num in my_vec.iter() {
            let number = Some(num.clone());
            let top = heap.pop();
            assert!(number == top);
            println!("{:?} == {:?}", number, top);
        }
    }
    println!("");
}

fn example_increase() {
    println!("example_increase (demonstrate the increase_key function)");
    let mut heap = HollowHeap::new();
    heap.push(1);
    let second = heap.push(2);
    heap.push(3);
    heap.increase_key(second, 5); // -> first value should be 5

    while let Some(node) = heap.pop() {
        println!("{:?}", node);
    }
    println!("");
}

fn example_complicated() {
    println!("example_complicated (lots of stuff happening :-D)");
    let vec1 = vec![1, -5, 6, 10, -555, 666, 100];
    let mut heap = HollowHeap::new();
    let mut five = None;
    {
        for num in vec1.into_iter() {
            if num == 666 {
                five = Some(heap.push(num.clone()));
            } else {
                heap.push(num.clone());
            }
        }
    }
    heap.increase_key(five.unwrap(), 777);
    println!("{:?}", heap.pop());
    println!("{:?}", heap.pop());
    let vec2 = vec![2, -55, 67, 110];
    {
        for num in vec2.iter() {
            let index = heap.push(num.clone());
            heap.increase_key(index, num.clone() + 20);
        }
    }
    while let Some(node) = heap.pop() {
        println!("{:?}", node);
    }
    println!("");
}

fn main() {
    example_one();
    example_vec();
    example_increase();
    example_complicated();
}
