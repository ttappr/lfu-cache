# Least Frequently Used Cache

This is an example project demonstrating the use of the 
[linked-vector](https://crates.io/crates/linked-vector) crate. 
This was originally a solution to a coding challenge on LeetCode.

A Least Frequently Used cache is implemented using a hash map and a linked 
vector of queues. The queues are also linked vectors. The cache is 
essentially one linked vector that holds nested linked vectors that each 
correspond to the number of times a key has been accessed.

When a new key is added to the queue, and it's already filled to capacity,
the least frequently used key is removed. When a key is accessed, it's 
frequency count is incremented, which means it's moved to the next queue
for the next higher frequency count.

Both `insert()` and `get()` are O(1) operations.

`incr_freq()` has an example of how to use a cursor to move to specific
nodes in the linked vector.

`insert()` and `remove_lfu()` have examples of how the linked vectors can
be accessed through the `LinkedVector` API.

The Cargo.toml manifest is set up to pull from the reqired GitHub repo branch
necessary to build and run the project.