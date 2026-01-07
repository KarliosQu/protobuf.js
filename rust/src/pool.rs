use parking_lot::Mutex;
use std::sync::Arc;
use once_cell::sync::Lazy;

pub static BUFFER_POOL: Lazy<BufferPool> = Lazy::new(|| BufferPool::new(1024));

/// Thread-safe buffer pool for reducing allocations
#[allow(dead_code)]
pub struct BufferPool {
    pools: Vec<Arc<Mutex<Vec<Vec<u8>>>>>,
    max_pool_size: usize,
    metrics: Arc<Mutex<PoolMetrics>>,
}

/// Metrics for buffer pool performance
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct PoolMetrics {
    pub hits: u64,
    pub misses: u64,
    pub total_acquired: u64,
    pub current_pooled: usize,
}

#[allow(dead_code)]
impl PoolMetrics {
    pub fn hit_rate(&self) -> f64 {
        if self.total_acquired == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_acquired as f64
        }
    }
}

#[allow(dead_code)]
impl BufferPool {
    /// Create a new buffer pool with maximum pool size per size class
    pub fn new(max_pool_size: usize) -> Self {
        // Size classes: 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768 bytes
        let num_classes = 10;
        let mut pools = Vec::with_capacity(num_classes);
        for _ in 0..num_classes {
            pools.push(Arc::new(Mutex::new(Vec::new())));
        }

        BufferPool {
            pools,
            max_pool_size,
            metrics: Arc::new(Mutex::new(PoolMetrics::default())),
        }
    }

    /// Acquire a buffer of at least min_size bytes
    pub fn acquire(&self, min_size: usize) -> PooledBuffer {
        let size_class = Self::size_class(min_size);
        let actual_size = Self::class_to_size(size_class);

        let mut metrics = self.metrics.lock();
        metrics.total_acquired += 1;

        if let Some(pool) = self.pools.get(size_class) {
            if let Some(buffer) = pool.lock().pop() {
                metrics.hits += 1;
                metrics.current_pooled -= 1;
                drop(metrics);
                return PooledBuffer {
                    buffer,
                    pool: Some(Arc::new(BufferPoolRef {
                        pool: self.pools[size_class].clone(),
                        max_pool_size: self.max_pool_size,
                        metrics: self.metrics.clone(),
                    })),
                };
            }
        }

        metrics.misses += 1;
        drop(metrics);

        // Create new buffer
        let buffer = vec![0u8; actual_size];
        PooledBuffer {
            buffer,
            pool: Some(Arc::new(BufferPoolRef {
                pool: self.pools[size_class].clone(),
                max_pool_size: self.max_pool_size,
                metrics: self.metrics.clone(),
            })),
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> PoolMetrics {
        self.metrics.lock().clone()
    }

    /// Determine size class for a given size (0-9)
    fn size_class(size: usize) -> usize {
        if size == 0 {
            return 0; // 64
        }
        let size = size.max(64);
        let class = (size as f64).log2().ceil() as usize;
        let class = if class < 6 { 0 } else { class - 6 }; // 2^6 = 64
        class.min(9)
    }

    fn class_to_size(class: usize) -> usize {
        64 << class
    }
}

pub struct PooledBuffer {
    buffer: Vec<u8>,
    pool: Option<Arc<BufferPoolRef>>,
}

struct BufferPoolRef {
    pool: Arc<Mutex<Vec<Vec<u8>>>>,
    max_pool_size: usize,
    metrics: Arc<Mutex<PoolMetrics>>,
}

impl PooledBuffer {
    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(pool_ref) = &self.pool {
            let mut pool = pool_ref.pool.lock();
            if pool.len() < pool_ref.max_pool_size {
                // Clear buffer but keep capacity
                self.buffer.clear();
                // We have to move out the buffer. Since we are in drop, self is &mut.
                // We can swap with an empty vector.
                let mut temp = Vec::new();
                std::mem::swap(&mut self.buffer, &mut temp);
                pool.push(temp);
                
                let mut metrics = pool_ref.metrics.lock();
                metrics.current_pooled += 1;
            }
        }
    }
}
