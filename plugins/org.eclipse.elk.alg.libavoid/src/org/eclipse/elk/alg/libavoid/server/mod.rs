use std::error::Error;
use std::fmt;
use std::sync::LazyLock;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup {
    Normal,
    Error,
    Stop,
}

#[derive(Debug)]
pub struct LibavoidServer {
    process_timeout: i32,
}

impl LibavoidServer {
    pub fn new() -> Self {
        LibavoidServer {
            process_timeout: 10000,
        }
    }

    pub fn initialize(&mut self) {}

    pub fn cleanup(&mut self, _mode: Cleanup) {}

    pub fn cancel_process(&mut self) {}

    pub fn set_process_timeout(&mut self, timeout: i32) {
        self.process_timeout = timeout;
    }

    pub fn process_timeout(&self) -> i32 {
        self.process_timeout
    }
}

impl Default for LibavoidServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LibavoidServerException {
    message: String,
}

impl LibavoidServerException {
    pub fn new(message: impl Into<String>) -> Self {
        LibavoidServerException {
            message: message.into(),
        }
    }
}

impl fmt::Display for LibavoidServerException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for LibavoidServerException {}

pub struct LibavoidServerPool {
    servers: Mutex<Vec<LibavoidServer>>,
}

impl LibavoidServerPool {
    pub fn instance() -> &'static LibavoidServerPool {
        &INSTANCE
    }

    pub fn fetch(&self) -> LibavoidServer {
        let mut servers = self.servers.lock().expect("libavoid server pool");
        servers.pop().unwrap_or_default()
    }

    pub fn release(&self, server: LibavoidServer) {
        let mut servers = self.servers.lock().expect("libavoid server pool");
        servers.push(server);
    }

    pub fn dispose(&self) {
        let mut servers = self.servers.lock().expect("libavoid server pool");
        servers.clear();
    }
}

static INSTANCE: LazyLock<LibavoidServerPool> = LazyLock::new(|| LibavoidServerPool {
    servers: Mutex::new(Vec::new()),
});
