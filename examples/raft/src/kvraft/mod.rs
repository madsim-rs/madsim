mod client;
mod msg;
mod server;
#[cfg(test)]
mod tester;
#[cfg(test)]
mod tests;

#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("no leader")]
    NoLeader,
}

pub type Result<T> = std::result::Result<T, Error>;
