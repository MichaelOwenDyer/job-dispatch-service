use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;
use std::path::Path;
use tokio::fs;
use tracing::error;

/// Load a JSON file and deserialize it into a `Vec<T>`.
/// The JSON file must contain a top-level JSON array.
/// Each element of the array is deserialized into a `T`; if deserialization fails, the element is skipped.
/// If the file does not exist, or if it is empty, an empty `Vec<T>` is returned.
async fn load<T: for<'de> Deserialize<'de>>(file: &Path) -> Vec<T> {
    fs::read_to_string(file)
        .await
        .ok()
        .and_then(|data| serde_json::from_str::<Vec<Value>>(&data).ok())
        .map(|vec| vec.into_iter().filter_map(|value| serde_json::from_value(value).ok()).collect())
        .unwrap_or_default()
}

/// Serialize a slice of Ts into a JSON string and save it to a file.
/// An error message is logged if the file cannot be written to.
/// # Panics
/// This function panics if the serialization impl for T fails.
/// It is easily verifiable at compile time that this will never happen.
async fn save<T: Serialize>(file: &Path, queue: &[T]) {
    if let Err(err) = fs::write(file, serde_json::to_string_pretty(queue).unwrap()).await {
        error!("Failed to save queue to file: {}", err);
    }
}

/// A queue backed by a JSON file.
/// Every operation on the queue reads from or writes to the file.
#[derive(Debug)]
pub struct JsonFileQueue<T> {
    file: Box<Path>, // Path is an unsized type, so we need to box it to store it on the heap
    _phantom: PhantomData<T>, // This field is needed to keep the type parameter T alive
}

impl<T> JsonFileQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Creates a new JsonFileQueue pointing to the given file path.
    pub fn new(file: impl AsRef<Path>) -> Self {
        Self {
            file: Box::from(file.as_ref()),
            _phantom: PhantomData,
        }
    }

    /// Removes and returns the first element of the queue, if there is one.
    /// This operation reads from and writes to the file.
    pub async fn dequeue(&mut self) -> Option<T> {
        let mut queue = load(&self.file).await;
        let item = queue.pop();
        save(&self.file, &queue).await;
        item
    }

    /// Appends an element to the end of the queue, and returns the new length of the queue.
    /// This operation reads from and writes to the file.
    pub async fn enqueue(&mut self, item: T) -> usize {
        let mut queue = load(&self.file).await;
        queue.push(item);
        save(&self.file, &queue).await;
        queue.len()
    }
}

/// A queue backed by a JSON file with an in-memory cache.
/// The cache is loaded once upon creation and is updated on every enqueue and dequeue operation.
/// Additionally, the file is written to on every enqueue and dequeue operation.
/// This is more performant than JsonFileQueue because it only reads from the file once,
/// but is more memory-intensive because it keeps the entire queue in memory.
#[derive(Debug)]
pub struct CachedJsonFileQueue<T> {
    file: Box<Path>,
    cache: Vec<T>,
}

impl<T> CachedJsonFileQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Creates a new CachedJsonFileQueue pointing to the given file path.
    /// The queue is loaded from the file upon creation.
    pub async fn new(file: impl AsRef<Path>) -> Self {
        let file = Box::from(file.as_ref());
        let cache = load(&file).await;
        Self { file, cache }
    }

    /// Removes and returns the first element of the queue, if there is one.
    /// This operation writes to the file.
    pub async fn dequeue(&mut self) -> Option<T> {
        let item = self.cache.pop();
        save(&self.file, &self.cache).await;
        item
    }

    /// Appends an element to the end of the queue, and returns the new length of the queue.
    /// This operation writes to the file.
    pub async fn enqueue(&mut self, item: T) -> usize {
        self.cache.push(item);
        save(&self.file, &self.cache).await;
        self.cache.len()
    }
}
