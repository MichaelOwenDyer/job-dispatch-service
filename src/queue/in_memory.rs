use std::collections::VecDeque;

/// A queue backed by an in-memory VecDeque.
/// This is the simplest and most performant queue implementation.
/// It is not persistent, so the queue is lost when the program exits.
#[derive(Debug)]
pub struct InMemoryQueue<T>(VecDeque<T>);

impl<T> InMemoryQueue<T> {
    /// Creates a new InMemoryQueue.
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Removes and returns the first element of the queue, if there is one.
    pub fn dequeue(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    /// Appends an element to the end of the queue, and returns the new length of the queue.
    pub fn enqueue(&mut self, item: T) -> usize {
        self.0.push_back(item);
        self.0.len()
    }
}