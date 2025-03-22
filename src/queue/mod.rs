//! Implementations for queuing jobs and workers.

mod in_memory;
mod json_file;

pub use in_memory::InMemoryQueue;
pub use json_file::CachedJsonFileQueue;
pub use json_file::JsonFileQueue;

/// A queue that is backed by one of the available implementations.
#[derive(Debug, derive_more::From)]
pub enum Queue<T> {
    InMemory(InMemoryQueue<T>),
    JsonFile(JsonFileQueue<T>),
    CachedJsonFile(CachedJsonFileQueue<T>),
}

impl<T> Queue<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    /// Removes and returns the first element of the queue, if there is one.
    pub async fn dequeue(&mut self) -> Option<T> {
        match self {
            Self::InMemory(queue) => queue.dequeue(),
            Self::JsonFile(queue) => queue.dequeue().await,
            Self::CachedJsonFile(queue) => queue.dequeue().await,
        }
    }
    /// Appends an element to the end of the queue, and returns the new length of the queue.
    pub async fn enqueue(&mut self, t: T) -> usize {
        match self {
            Self::InMemory(queue) => queue.enqueue(t),
            Self::JsonFile(queue) => queue.enqueue(t).await,
            Self::CachedJsonFile(queue) => queue.enqueue(t).await,
        }
    }
}