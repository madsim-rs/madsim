use self::{logs::Logs, msg::*};
use futures::{channel::mpsc, join, select, stream::FuturesUnordered, FutureExt, StreamExt};
use madsim::{
    fs::{self, File},
    net::NetworkLocalHandle,
    rand::{self, Rng},
    task::Task,
    time::{self, *},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    future::Future,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, Weak},
};

mod logs;
mod msg;

#[derive(Clone)]
pub struct RaftHandle {
    inner: Arc<Mutex<Raft>>,
}

type MsgSender = mpsc::UnboundedSender<ApplyMsg>;
pub type MsgRecver = mpsc::UnboundedReceiver<ApplyMsg>;

/// As each Raft peer becomes aware that successive log entries are committed,
/// the peer should send an `ApplyMsg` to the service (or tester) on the same
/// server, via the `apply_ch` passed to `Raft::new`.
pub enum ApplyMsg {
    Command {
        data: Vec<u8>,
        index: u64,
    },
    // For 2D:
    Snapshot {
        data: Vec<u8>,
        term: u64,
        index: u64,
    },
}

#[derive(Debug)]
pub struct Start {
    /// The index that the command will appear at if it's ever committed.
    pub index: u64,
    /// The current term.
    pub term: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("this node is not a leader")]
    NotLeader,
    #[error("IO error")]
    IO(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

struct Raft {
    peers: Vec<SocketAddr>,
    me: usize,

    // Your data here (2A, 2B, 2C).
    // Look at the paper's Figure 2 for a description of what
    // state a Raft server must maintain.
    apply_ch: MsgSender,
    this: Weak<Mutex<Self>>,

    /// Tasks of current state.
    /// On state change, drop these tasks to cancel them.
    tasks_of_state: Vec<Task<()>>,

    state: State,
    // State Persistent state on all servers
    voted_for: Option<usize>,
    // NOTE: the first log is the last included one in the snapshot
    log: Logs,
    // Volatile state on all servers
    commit_index: usize,
    last_applied: usize,
    // Volatile state on leaders
    next_index: Vec<usize>,
    match_index: Vec<usize>,

    snapshot: Vec<u8>,

    last_apply_entries_received: Instant,
}

/// State of a raft peer.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct State {
    term: u64,
    role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Follower,
    Candidate,
    Leader,
}

impl Default for Role {
    fn default() -> Self {
        Role::Follower
    }
}

impl State {
    fn is_leader(&self) -> bool {
        matches!(self.role, Role::Leader)
    }
}

/// Data needs to be persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Persist {
    term: u64,
    voted_for: Option<usize>,
    log: Logs,
}

impl fmt::Debug for Raft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Raft({},t={},l=[{},{}],{:?})",
            self.me,
            self.state.term,
            self.log.begin(),
            self.log.len() - 1,
            self.state.role
        )
    }
}

macro_rules! try_upgrade {
    ($weak:expr) => {
        match $weak.upgrade() {
            Some(t) => t,
            None => return,
        }
    };
}

macro_rules! lock_and_check_raft {
    ($this:expr, $state:expr) => {{
        let lock = $this.lock().unwrap();
        if lock.state != $state {
            return;
        }
        lock
    }};
}

impl RaftHandle {
    pub async fn new(peers: Vec<SocketAddr>, me: usize) -> (Self, MsgRecver) {
        let n = peers.len();
        let (apply_ch, recver) = mpsc::unbounded();
        let inner = Arc::new(Mutex::new(Raft {
            peers,
            me,
            apply_ch,
            this: Weak::default(),
            tasks_of_state: vec![],
            state: State::default(),
            voted_for: None,
            log: Logs::new(),
            commit_index: 0,
            last_applied: 0,
            next_index: vec![0; n],
            match_index: vec![0; n],
            snapshot: vec![],
            last_apply_entries_received: Instant::now(),
        }));
        let handle = RaftHandle { inner };
        // initialize from state persisted before a crash
        handle.restore().await.expect("failed to restore");
        handle.start_rpc_server();

        let mut raft = handle.inner.lock().unwrap();
        raft.this = Arc::downgrade(&handle.inner);
        raft.spawn_follower_timer();
        info!("{:?} created", raft);
        drop(raft);

        (handle, recver)
    }

    /// Start agreement on the next command to be appended to Raft's log.
    ///
    /// If this server isn't the leader, returns [`Error::NotLeader`].
    /// Otherwise start the agreement and return immediately.
    ///
    /// There is no guarantee that this command will ever be committed to the
    /// Raft log, since the leader may fail or lose an election.
    pub async fn start(&self, cmd: &[u8]) -> Result<Start> {
        let (ret, persist) = {
            let mut raft = self.inner.lock().unwrap();
            let ret = raft.start(cmd);
            let persist = ret.as_ref().ok().map(|_| raft.persist());
            (ret, persist)
        };
        if let Some(persist) = persist {
            persist.await?;
        }
        ret
    }

    /// The current term of this peer.
    pub fn term(&self) -> u64 {
        let raft = self.inner.lock().unwrap();
        raft.state.term
    }

    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        let raft = self.inner.lock().unwrap();
        raft.state.is_leader()
    }

    /// A service wants to switch to snapshot.  
    ///
    /// Only do so if Raft hasn't have more recent info since it communicate
    /// the snapshot on `apply_ch`.
    pub async fn cond_install_snapshot(
        &self,
        _last_included_term: u64,
        _last_included_index: u64,
        _snapshot: &[u8],
    ) -> bool {
        true
    }

    /// The service says it has created a snapshot that has all info up to and
    /// including index. This means the service no longer needs the log through
    /// (and including) that index. Raft should now trim its log as much as
    /// possible.
    pub async fn snapshot(&self, index: u64, snapshot: &[u8]) -> Result<()> {
        let persist = {
            let mut this = self.inner.lock().unwrap();
            info!("{:?} snapshot at index {}", this, index);
            this.log.trim_start_until(index as usize);
            this.snapshot = snapshot.into();
            this.persist_with_snapshot()
        };
        persist.await?;
        Ok(())
    }

    /// Restore previously persisted state.
    async fn restore(&self) -> io::Result<()> {
        match fs::read("snapshot").await {
            Ok(snapshot) => {
                let mut this = self.inner.lock().unwrap();
                this.snapshot = snapshot;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        match fs::read("state").await {
            Ok(state) => {
                let persist: Persist = flexbuffers::from_slice(&state).unwrap();
                let mut this = self.inner.lock().unwrap();
                this.state.term = persist.term;
                this.voted_for = persist.voted_for;
                this.log = persist.log;
                let commit_index = this.log.begin();
                // NOTE: should load snapshot before update_commit
                this.update_commit(commit_index);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    fn start_rpc_server(&self) {
        let net = NetworkLocalHandle::current();

        let this = self.clone();
        net.add_rpc_handler(move |args: RequestVoteArgs| {
            let this = this.clone();
            async move { this.request_vote(args).await.unwrap() }
        });

        let this = self.clone();
        net.add_rpc_handler(move |args: AppendEntriesArgs| {
            let this = this.clone();
            async move { this.append_entries(args).await.unwrap() }
        });

        let this = self.clone();
        net.add_rpc_handler(move |args: InstallSnapshotArgs| {
            let this = this.clone();
            async move { this.install_snapshot(args).await.unwrap() }
        });
    }

    async fn request_vote(&self, args: RequestVoteArgs) -> Result<RequestVoteReply> {
        let (reply, persist) = {
            let mut this = self.inner.lock().unwrap();
            let reply = this.request_vote(args);
            let persist = this.persist();
            (reply, persist)
        };
        persist.await?;
        Ok(reply)
    }

    async fn append_entries(&self, args: AppendEntriesArgs) -> Result<AppendEntriesReply> {
        let (reply, persist) = {
            let mut this = self.inner.lock().unwrap();
            let reply = this.append_entries(args);
            let persist = this.persist();
            (reply, persist)
        };
        persist.await?;
        Ok(reply)
    }

    async fn install_snapshot(&self, args: InstallSnapshotArgs) -> Result<InstallSnapshotReply> {
        let (reply, persist) = {
            let mut this = self.inner.lock().unwrap();
            let reply = this.install_snapshot(args);
            let persist = this.persist_with_snapshot();
            (reply, persist)
        };
        persist.await?;
        Ok(reply)
    }
}

impl Raft {
    /// save Raft's persistent state to stable storage,
    /// where it can later be retrieved after a crash and restart.
    /// see paper's Figure 2 for a description of what should be persistent.
    fn persist(&self) -> impl Future<Output = io::Result<()>> {
        // Your code here (2C).
        debug!("persist");
        let persist = Persist {
            term: self.state.term,
            voted_for: self.voted_for,
            log: self.log.clone(),
        };
        let data = flexbuffers::to_vec(&persist).unwrap();

        async move {
            // FIXME: crash on partial write
            let file = File::create("state").await?;
            file.write_all_at(&data, 0).await?;
            file.sync_all().await?;
            Ok(())
        }
    }

    fn persist_with_snapshot(&self) -> impl Future<Output = io::Result<()>> {
        let snapshot = self.snapshot.clone();
        let f1 = self.persist();
        let f2 = async move {
            // FIXME: crash on partial write
            let file = File::create("snapshot").await?;
            file.write_all_at(&snapshot, 0).await?;
            file.sync_all().await?;
            Ok(()) as io::Result<()>
        };
        async move {
            let (r1, r2) = join!(f1, f2);
            r1?;
            r2?;
            Ok(())
        }
    }

    fn start(&mut self, data: &[u8]) -> Result<Start> {
        // Your code here (2B).
        if !self.state.is_leader() {
            return Err(Error::NotLeader);
        }
        let index = self.log.len() as u64;
        let term = self.state.term;
        self.log.push(Log {
            term,
            data: data.into(),
        });
        info!("{:?} start index {}", self, index);
        Ok(Start { index, term })
    }

    // WARN: You may need to persist the state manually.
    // FIXME: missing persist at some point
    fn transfer_state(&mut self, term: u64, role: Role, reason: &str) {
        assert!(term >= self.state.term);
        assert_ne!(self.state, State { term, role });
        info!("{:?} {} => t={},{:?}", self, reason, term, role);
        // cancel all tasks
        self.tasks_of_state.clear();
        if term > self.state.term {
            self.voted_for = None;
        }
        self.state = State { term, role };
        match role {
            Role::Follower => {
                self.last_apply_entries_received = Instant::now();
                self.spawn_follower_timer();
            }
            Role::Candidate => {
                self.voted_for = Some(self.me);
                self.spawn_candidate_task();
            }
            Role::Leader => {
                self.next_index.fill(self.log.len());
                self.match_index.fill(0);
                self.spawn_leader_append_entries();
            }
        }
    }

    fn spawn_follower_timer(&mut self) {
        let this = self.this.clone();
        let state = self.state;
        let timeout = Self::generate_election_timeout();
        self.tasks_of_state.push(madsim::task::spawn(async move {
            loop {
                let deadline = {
                    let this = try_upgrade!(this);
                    let mut this = lock_and_check_raft!(this, state);
                    if this.last_apply_entries_received.elapsed() >= timeout {
                        this.transfer_state(state.term + 1, Role::Candidate, "election timeout");
                        return;
                    }
                    this.last_apply_entries_received + timeout
                };
                time::sleep_until(deadline).await;
            }
        }));
    }

    fn spawn_candidate_task(&mut self) {
        let this = self.this.clone();
        let state = self.state;
        let args = RequestVoteArgs {
            term: self.state.term,
            candidate_id: self.me as u64,
            last_log_index: self.log.len() as u64 - 1,
            last_log_term: self.log.last().unwrap().term,
        };
        debug!("{:?} -> {:?}", self, args);
        let net = NetworkLocalHandle::current();
        let mut rpcs = self
            .peers
            .iter()
            .enumerate()
            .filter(|&(idx, _)| idx != self.me)
            .map(|(_, &peer)| {
                net.clone()
                    .call_owned::<_, RequestVoteReply>(peer, args.clone())
            })
            .collect::<FuturesUnordered<_>>();
        let deadline = Instant::now() + Self::generate_election_timeout();
        let min_vote = (self.peers.len() + 1) / 2;
        self.tasks_of_state.push(madsim::task::spawn(async move {
            let mut vote_count = 1;
            loop {
                #[derive(Debug)]
                enum Event {
                    Reply(RequestVoteReply),
                    RpcError,
                    AllComplete,
                    Timeout,
                }
                let event = select! {
                    _ = time::sleep_until(deadline).fuse() => Event::Timeout,
                    ret = rpcs.next() => match ret {
                        None => Event::AllComplete,
                        Some(Ok(reply)) => Event::Reply(reply),
                        Some(Err(_)) => Event::RpcError,
                    }
                };
                let this = try_upgrade!(this);
                let mut this = lock_and_check_raft!(this, state);
                match event {
                    Event::Reply(reply) => {
                        assert!(reply.term >= state.term);
                        if reply.term > state.term {
                            this.transfer_state(reply.term, Role::Follower, "higher term (->RV)");
                            return;
                        }
                        if reply.vote_granted {
                            vote_count += 1;
                            if vote_count >= min_vote {
                                this.transfer_state(state.term, Role::Leader, "win election");
                                return;
                            }
                        }
                    }
                    Event::RpcError => {}
                    Event::Timeout => {
                        this.transfer_state(state.term + 1, Role::Candidate, "election timeout");
                        return;
                    }
                    Event::AllComplete => {}
                }
            }
        }));
    }

    fn spawn_leader_append_entries(&mut self) {
        for (i, &peer) in self.peers.iter().enumerate() {
            if i == self.me {
                continue;
            }
            let this = self.this.clone();
            let state = self.state;
            let leader_id = self.me as u64;
            self.tasks_of_state.push(madsim::task::spawn(async move {
                let mut backoff = 1;
                let net = NetworkLocalHandle::current();
                loop {
                    enum Task {
                        AppendEntries(AppendEntriesArgs),
                        InstallSnapshot(InstallSnapshotArgs),
                    }
                    let (match_index_if_success, task) = {
                        let this = try_upgrade!(this);
                        let this = lock_and_check_raft!(this, state);
                        let next_index = this.next_index[i];
                        let prev_log_index = next_index - 1;
                        if prev_log_index < this.log.begin() {
                            let args = InstallSnapshotArgs {
                                term: state.term,
                                leader_id,
                                last_included_index: this.log.begin() as u64,
                                last_included_term: this.log[this.log.begin()].term,
                                data: this.snapshot.clone(),
                            };
                            debug!("{:?} ->{} {:?}", *this, i, args);
                            (this.log.begin(), Task::InstallSnapshot(args))
                        } else {
                            let args = AppendEntriesArgs {
                                term: state.term,
                                leader_id,
                                prev_log_index: prev_log_index as u64,
                                prev_log_term: this.log[prev_log_index].term,
                                entries: this.log[next_index..].into(),
                                leader_commit: this.commit_index as u64,
                            };
                            debug!("{:?} ->{} {:?}", *this, i, args);
                            (
                                prev_log_index + args.entries.len(),
                                Task::AppendEntries(args),
                            )
                        }
                    };
                    match task {
                        Task::InstallSnapshot(args) => {
                            let reply = select! {
                                ret = net.clone().call_owned::<_, InstallSnapshotReply>(peer, args).fuse() => match ret {
                                    Err(_) => continue,
                                    Ok(x) => x,
                                },
                                _ = time::sleep(Self::generate_heartbeat_interval()).fuse() => continue,
                            };
                            let this = try_upgrade!(this);
                            let mut this = lock_and_check_raft!(this, state);
                            // If RPC request or response contains term T > currentTerm:
                            // set currentTerm = T, convert to follower (§5.1)
                            if reply.term > this.state.term {
                                this.transfer_state(
                                    reply.term,
                                    Role::Follower,
                                    "higher term (->IS)",
                                );
                                return;
                            }
                            this.next_index[i] = match_index_if_success + 1;
                            this.match_index[i] = match_index_if_success;
                            this.update_commit_from_match();
                            backoff = 1;
                            continue;
                        }
                        Task::AppendEntries(args) => {
                            let reply = select! {
                                ret = net.clone().call_owned::<_, AppendEntriesReply>(peer, args).fuse() => match ret {
                                    Err(_) => continue,
                                    Ok(x) => x,
                                },
                                _ = time::sleep(Self::generate_heartbeat_interval()).fuse() => continue,
                            };
                            let this = try_upgrade!(this);
                            let mut this = lock_and_check_raft!(this, state);
                            // If RPC request or response contains term T > currentTerm:
                            // set currentTerm = T, convert to follower (§5.1)
                            if reply.term > this.state.term {
                                this.transfer_state(
                                    reply.term,
                                    Role::Follower,
                                    "higher term (->AE)",
                                );
                                return;
                            }
                            if reply.success {
                                // If successful: update nextIndex and matchIndex for follower (§5.3)
                                this.next_index[i] = match_index_if_success + 1;
                                this.match_index[i] = match_index_if_success;
                                this.update_commit_from_match();
                                backoff = 1;
                            } else {
                                // If AppendEntries fails because of log inconsistency:
                                // decrement nextIndex and retry (§5.3)
                                if backoff >= this.next_index[i] {
                                    this.next_index[i] = 1;
                                } else {
                                    this.next_index[i] -= backoff;
                                }
                                backoff *= 2;
                                continue;
                            }
                        }
                    }
                    time::sleep(Self::generate_heartbeat_interval()).await;
                }
            }));
        }
    }

    fn update_commit_from_match(&mut self) {
        // If there exists an N such that N > commitIndex, a majority of matchIndex[i] >= N,
        // and log[N].term == currentTerm: set commitIndex = N (§5.3, §5.4).
        assert!(self.state.is_leader());
        let mut match_index = self.match_index.clone();
        match_index.sort_unstable();
        let majority = self.peers.len() - self.peers.len() / 2;
        let commit_index = match_index[majority];
        if commit_index <= self.commit_index || self.log[commit_index].term != self.state.term {
            return;
        }
        self.update_commit(commit_index);
    }

    fn update_commit(&mut self, commit_index: usize) {
        if commit_index <= self.commit_index {
            return;
        }
        self.commit_index = commit_index;
        info!("{:?} commit to index {}", self, commit_index);
        self.apply();
    }

    fn apply(&mut self) {
        // If commitIndex > lastApplied: increment lastApplied,
        // apply log[lastApplied] to state machine (§5.3)
        while self.commit_index > self.last_applied {
            let mut i = self.last_applied + 1;
            if i <= self.log.begin() {
                i = self.log.begin();
                self.apply_ch
                    .unbounded_send(ApplyMsg::Snapshot {
                        data: self.snapshot.clone(),
                        index: i as u64,
                        term: self.log[i].term,
                    })
                    .unwrap();
            } else {
                self.apply_ch
                    .unbounded_send(ApplyMsg::Command {
                        data: self.log[i].data.clone(),
                        index: i as u64,
                    })
                    .unwrap();
            }
            self.last_applied = i;
        }
    }

    fn generate_election_timeout() -> Duration {
        Duration::from_millis(rand::rng().gen_range(150..300))
    }

    fn generate_heartbeat_interval() -> Duration {
        Duration::from_millis(50)
    }

    fn request_vote(&mut self, args: RequestVoteArgs) -> RequestVoteReply {
        // Your code here (2A, 2B).
        // Reply false if term < currentTerm (§5.1)
        if args.term < self.state.term {
            debug!("{:?} <- deny {:?}", self, args);
            return RequestVoteReply {
                term: self.state.term,
                vote_granted: false,
            };
        }
        // If RPC request or response contains term T > currentTerm:
        // set currentTerm = T, convert to follower (§5.1)
        if args.term > self.state.term {
            self.transfer_state(args.term, Role::Follower, "higher term (<-RV)");
        }
        // If votedFor is null or candidateId, and candidate’s log is at least
        // as up-to-date as receiver’s log, grant vote (§5.2, §5.4)
        let can_vote =
            self.voted_for.is_none() || self.voted_for == Some(args.candidate_id as usize);
        let log_up_to_date = (args.last_log_term, args.last_log_index)
            >= (self.log.last().unwrap().term, self.log.len() as u64 - 1);
        let vote_granted = can_vote && log_up_to_date;
        if vote_granted {
            self.voted_for = Some(args.candidate_id as usize);
        }
        debug!(
            "{:?} <- {} {:?}",
            self,
            if vote_granted { "accept" } else { "deny" },
            args
        );
        RequestVoteReply {
            term: self.state.term,
            vote_granted,
        }
    }

    fn append_entries(&mut self, args: AppendEntriesArgs) -> AppendEntriesReply {
        // Reply false if term < currentTerm (§5.1)
        if args.term < self.state.term {
            debug!("{:?} <- deny {:?}", self, args);
            return AppendEntriesReply {
                term: self.state.term,
                success: false,
            };
        }
        // If RPC request or response contains term T > currentTerm:
        // set currentTerm = T, convert to follower (§5.1)
        if args.term > self.state.term || self.state.role == Role::Candidate {
            self.transfer_state(args.term, Role::Follower, "higher term (<-AE)");
        }
        self.last_apply_entries_received = Instant::now();
        // Reply false if log doesn’t contain an entry at prevLogIndex
        // whose term matches prevLogTerm (§5.3)
        if !matches!(self.log.get(args.prev_log_index as usize), Some(log) if log.term == args.prev_log_term)
        {
            debug!("{:?} <- deny {:?}", self, args);
            return AppendEntriesReply {
                term: self.state.term,
                success: false,
            };
        }
        // If an existing entry conflicts with a new one (same index but different terms),
        // delete the existing entry and all that follow it (§5.3)
        let log_offset = args.prev_log_index as usize + 1;
        if let Some(idx) = args
            .entries
            .iter()
            .zip(self.log[log_offset..].iter())
            .position(|(remote, local)| remote.term != local.term)
        {
            self.log.truncate(log_offset + idx);
        }
        // Append any new entries not already in the log
        if log_offset + args.entries.len() > self.log.len() {
            let new_logs = &args.entries[self.log.len() - log_offset..];
            self.log.extend_from_slice(new_logs);
        }
        // If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)
        if args.leader_commit as usize > self.commit_index {
            let commit_index = (args.leader_commit as usize).min(self.log.len() - 1);
            self.update_commit(commit_index);
        }
        debug!("{:?} <- accept {:?}", self, args);
        AppendEntriesReply {
            term: self.state.term,
            success: true,
        }
    }

    fn install_snapshot(&mut self, args: InstallSnapshotArgs) -> InstallSnapshotReply {
        debug!("{:?} <- {:?}", self, args);
        // 1. Reply immediately if term < currentTerm
        if args.term < self.state.term {
            return InstallSnapshotReply {
                term: self.state.term,
            };
        }
        self.last_apply_entries_received = Instant::now();
        // 0. If RPC request or response contains term T > currentTerm:
        // set currentTerm = T, convert to follower (§5.1)
        if args.term > self.state.term {
            self.transfer_state(args.term, Role::Follower, "higher term (<-IS)");
        }
        // 2. Create new snapshot file if first chunk (offset is 0)
        // 3. Write data into snapshot file at given offset
        // 4. Reply and wait for more data chunks if done is false
        // 5. Save snapshot file, discard any existing or partial snapshot with a smaller index
        // 6. If existing log entry has same index and term as snapshot’s last included entry, retain log entries following it and reply
        let prev_index = args.last_included_index as usize;
        if matches!(self.log.get(prev_index), Some(log) if log.term == args.last_included_term) {
            self.log.trim_start_until(prev_index);
        } else {
            // 7. Discard the entire log
            self.log.clear();
            self.log.trim_start_until(prev_index);
            self.log.push(Log {
                term: args.last_included_term,
                data: vec![],
            });
        }
        self.snapshot = args.data;
        // 8. Reset state machine using snapshot contents (and load snapshot’s cluster configuration)
        self.update_commit(prev_index);

        InstallSnapshotReply {
            term: self.state.term,
        }
    }
}
