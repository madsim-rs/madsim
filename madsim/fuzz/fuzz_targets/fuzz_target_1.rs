#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // madsim::runtime::init_logger();
    let data = data.to_vec();
    std::thread::spawn(move || {
        let rt = madsim::runtime::Runtime::with_data_and_config(&data, madsim::Config::default());
        rt.try_block_on(main());
    })
    .join().unwrap();
});

use madsim::rand::{thread_rng, Rng};
use std::{cell::UnsafeCell, sync::Arc, time::Duration};

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
    madsim::time::sleep(Duration::from_millis(100)).await;
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
        // tracing::info!("begin({x}, {y})");
        // let t = thread_rng().gen_range(0..10);
        // madsim::time::sleep(Duration::from_micros(t)).await;
        madsim::task::yield_now().await;
        a()[i] = y;
        a()[j] = x;
        // tracing::info!("end  ({x}, {y})");
    }

    /// Check the invariant.
    fn check(&self) {
        let mut a = unsafe { &*self.x.get() }.clone();
        a.sort();
        assert_eq!(a, (0..a.len()).collect::<Vec<_>>());
    }
}
