use super::{Config, Runtime};
use futures_util::{stream, StreamExt};
use std::future::Future;
use std::time::{Duration, SystemTime};

/// Builds Madsim Runtime with custom configuration values.
pub struct Builder {
    /// The random seed for test.
    pub seed: u64,
    /// The number of tests.
    pub count: u64,
    /// The number of jobs to run simultaneously.
    pub jobs: u16,
    /// The configuration.
    pub config: Config,
    /// The time limit for the test.
    pub time_limit: Option<Duration>,
    /// Enable determinism check.
    pub check: bool,
}

impl Builder {
    /// Create a new builder from the following environment variables:
    ///
    /// - `MADSIM_TEST_SEED`: Set the random seed for test.
    ///
    ///     By default, the seed is set to the seconds since the Unix epoch.
    ///
    /// - `MADSIM_TEST_NUM`: Set the number of tests.
    ///
    ///     The seed will increase by 1 for each test.
    ///
    ///     By default, the number is 1.
    ///
    /// - `MADSIM_TEST_JOBS`: Set the number of jobs to run simultaneously.
    ///
    ///     By default, the number of jobs is 1.
    ///
    /// - `MADSIM_TEST_CONFIG`: Set the config file path.
    ///
    ///     By default, tests will use the default configuration.
    ///
    /// - `MADSIM_TEST_TIME_LIMIT`: Set the time limit for the test.
    ///
    ///     The test will panic if time limit exceeded in the simulation.
    ///
    ///     By default, there is no time limit.
    ///
    /// - `MADSIM_TEST_CHECK_DETERMINISM`: Enable determinism check.
    ///
    ///     The test will be run at least twice with the same seed.
    ///     If any non-determinism detected, it will panic as soon as possible.
    ///
    ///     By default, it is disabled.
    pub fn from_env() -> Self {
        let seed: u64 = if let Ok(seed_str) = std::env::var("MADSIM_TEST_SEED") {
            seed_str
                .parse()
                .expect("MADSIM_TEST_SEED should be an integer")
        } else {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        };
        let jobs: u16 = if let Ok(jobs_str) = std::env::var("MADSIM_TEST_JOBS") {
            jobs_str
                .parse()
                .expect("MADSIM_TEST_JOBS should be an integer")
        } else {
            1
        };
        let config = if let Ok(config_path) = std::env::var("MADSIM_TEST_CONFIG") {
            let content = std::fs::read_to_string(config_path).expect("failed to read config file");
            content
                .parse::<Config>()
                .expect("failed to parse config file")
        } else {
            Config::default()
        };
        let mut count: u64 = if let Ok(num_str) = std::env::var("MADSIM_TEST_NUM") {
            num_str
                .parse()
                .expect("MADSIM_TEST_NUM should be an integer")
        } else {
            1
        };
        let time_limit = std::env::var("MADSIM_TEST_TIME_LIMIT").ok().map(|num_str| {
            Duration::from_secs_f64(
                num_str
                    .parse::<f64>()
                    .expect("MADSIM_TEST_TIME_LIMIT should be an number"),
            )
        });
        let check = std::env::var("MADSIM_TEST_CHECK_DETERMINISM").is_ok();
        if check {
            count = count.max(2);
        }
        Builder {
            seed,
            count,
            jobs,
            config,
            time_limit,
            check,
        }
    }

    /// Run the future with configurations.
    pub fn run<F>(self, f: fn() -> F) -> F::Output
    where
        F: Future + 'static,
        F::Output: Send,
    {
        if self.check {
            return Runtime::check_determinism(self.seed, self.config, f);
        }
        let mut stream = stream::iter(self.seed..self.seed + self.count)
            .map(|seed| {
                let config = self.config.clone();
                async move {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let handle = std::thread::spawn(move || {
                        let mut rt = Runtime::with_seed_and_config(seed, config);
                        if let Some(limit) = self.time_limit {
                            rt.set_time_limit(limit);
                        }
                        let ret = rt.block_on(f());
                        tx.send(()).unwrap();
                        ret
                    });
                    let _ = rx.await;
                    (seed, handle.join())
                }
            })
            .buffer_unordered(self.jobs as usize);
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let mut return_value = None;
        while let Some((seed, res)) = rt.block_on(stream.next()) {
            match res {
                Ok(ret) => return_value = Some(ret),
                Err(e) => super::panic_with_info(seed, self.config.hash(), e),
            }
        }
        return_value.unwrap()
    }
}
