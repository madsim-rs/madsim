use madsim::rand::thread_rng;
use rand::Rng;
use std::{cell::UnsafeCell, sync::Arc, time::Duration};

#[madsim::main]
async fn main() {
    let a = Arc::new(Array::new(10));
    for _ in 0..3 {
        let a = a.clone();
        madsim::task::spawn(async move {
            loop {
                let x = thread_rng().gen_range(0..1000);
                madsim::time::sleep(Duration::from_micros(x)).await;
                let i = thread_rng().gen_range(0..10);
                let j = thread_rng().gen_range(0..10);
                a.swap(i, j).await;
                a.check();
            }
        });
    }
    std::future::pending::<()>().await;
}

struct Array {
    x: UnsafeCell<Vec<usize>>,
}

unsafe impl Send for Array {}
unsafe impl Sync for Array {}

impl Array {
    fn new(n: usize) -> Self {
        Self {
            x: UnsafeCell::new((0..n).collect()),
        }
    }

    /// Swap 2 elements in the array.
    async fn swap(&self, i: usize, j: usize) {
        let a = || unsafe { &mut *self.x.get() };
        let x = a()[i];
        let y = a()[j];
        tracing::info!("begin({x}, {y})");
        let t = thread_rng().gen_range(0..10);
        madsim::time::sleep(Duration::from_micros(t)).await;
        a()[i] = y;
        a()[j] = x;
        tracing::info!("end  ({x}, {y})");
    }

    /// Check the invariant.
    fn check(&self) {
        let mut a = unsafe { &*self.x.get() }.clone();
        a.sort();
        assert_eq!(a, (0..a.len()).collect::<Vec<_>>());
    }
}
