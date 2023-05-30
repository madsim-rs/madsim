#[cfg(not(madsim))]
pub use tokio::*;

#[cfg(madsim)]
pub use self::sim::*;
#[cfg(madsim)]
mod sim {
    // simulated API
    pub use madsim::net;
    #[cfg(feature = "signal")]
    pub use madsim::signal;
    #[cfg(feature = "rt")]
    pub use madsim::task::spawn;
    #[cfg(feature = "time")]
    pub use madsim::time;
    #[cfg(all(feature = "rt", feature = "macros"))]
    pub use madsim::{tokio_main as main, tokio_test as test};
    // for macro use
    #[cfg(all(feature = "rt", feature = "macros"))]
    #[doc(hidden)]
    pub use madsim;
    #[cfg(feature = "rt")]
    pub mod runtime;

    pub mod task {
        #[cfg(tokio_unstable)]
        pub use madsim::task::yield_now as consume_budget;

        pub use madsim::task::*;
        #[cfg(feature = "rt")]
        pub use tokio::task::LocalKey;
        #[cfg(feature = "rt")]
        pub mod futures {
            pub use tokio::task::futures::TaskLocalFuture;
        }
    }

    // not simulated API
    // TODO: simulate `fs`
    #[cfg(feature = "fs")]
    pub use tokio::fs;
    #[cfg(feature = "process")]
    pub use tokio::process;
    #[cfg(feature = "sync")]
    pub use tokio::sync;
    #[cfg(feature = "rt")]
    pub use tokio::task_local;
    pub use tokio::{io, pin};
    #[cfg(feature = "macros")]
    pub use tokio::{join, select, try_join};
}
