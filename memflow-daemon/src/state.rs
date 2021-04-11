use crate::error::{Error, Result};

use std::collections::HashMap;
use tokio::sync::{Mutex, MutexGuard};

use lazy_static::lazy_static;
use uuid::Uuid;

use memflow::prelude::v1::*;

lazy_static! {
    pub static ref STATE: Mutex<State> = Mutex::new(State::new());
}

pub fn state_lock_sync<'a>() -> MutexGuard<'a, State> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(STATE.lock())
}

pub fn new_uuid() -> String {
    let uuid = Uuid::new_v4();
    uuid.to_simple()
        .encode_lower(&mut Uuid::encode_buffer())
        .chars()
        .take(10)
        .collect::<String>()
}

/// Contains the entire global state of the daemon.
pub struct State {
    pub connections: HashMap<String, OpenedConnection>,
    pub connection_aliases: HashMap<String, String>,

    pub file_systems: HashMap<String, FileSystemHandle>,
    pub gdb_stubs: HashMap<String, GdbStubHandle>,
}

impl State {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            connection_aliases: HashMap::new(),

            file_systems: HashMap::new(),
            gdb_stubs: HashMap::new(),
        }
    }

    pub fn connection_add(
        &mut self,
        name: &str,
        alias: Option<String>,
        connection: Connection,
    ) -> Result<String> {
        if alias.is_some()
            && self
                .connection_aliases
                .contains_key(alias.as_ref().unwrap())
        {
            return Err(Error::Connector(
                "a connection with this alias already exists".into(),
            ));
        }

        let id = new_uuid();
        let conn = OpenedConnection::new(&id, name, alias.clone(), connection);

        self.connections.insert(id.clone(), conn);
        if let Some(a) = alias {
            self.connection_aliases.insert(a, id.clone());
        }

        Ok(id)
    }

    pub fn connection(&self, id: &str) -> Option<&OpenedConnection> {
        // first try to get by id
        if self.connections.contains_key(id) {
            self.connections.get(id)
        } else if let Some(real_id) = self.connection_aliases.get(id) {
            self.connections.get(real_id)
        } else {
            None
        }
    }

    pub fn connection_mut(&mut self, id: &str) -> Option<&mut OpenedConnection> {
        // first try to get by id
        if self.connections.contains_key(id) {
            self.connections.get_mut(id)
        } else if let Some(real_id) = self.connection_aliases.get(id) {
            self.connections.get_mut(real_id)
        } else {
            None
        }
    }

    pub fn connection_remove(&mut self, id: &str) -> Result<()> {
        let (id, alias) = if let Some(conn) = self.connection(id) {
            if conn.refcount == 0 {
                (conn.id.clone(), conn.alias.clone())
            } else {
                return Err(Error::Connector(
                    "connection still has open references".into(),
                ));
            }
        } else {
            return Err(Error::Connector("connection not found".into()));
        };

        if let Some(alias) = &alias {
            self.connection_aliases.remove(alias);
        }
        self.connections.remove(&id);

        Ok(())
    }
}

#[derive(Clone)]
pub enum Connection {
    Connector(ConnectorInstance),
    Os(OsInstance),
}

pub struct OpenedConnection {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub refcount: usize,
    pub connection: Connection,
}

impl OpenedConnection {
    pub fn new(id: &str, name: &str, alias: Option<String>, connection: Connection) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            alias,
            refcount: 0,
            connection,
        }
    }
}

pub struct FileSystemHandle {
    pub id: String,
    pub conn_id: String,
    pub mount_point: String,
}

impl FileSystemHandle {
    pub fn new(id: &str, conn_id: &str, mount_point: &str) -> Self {
        Self {
            id: id.to_string(),
            conn_id: conn_id.to_string(),
            mount_point: mount_point.to_string(),
        }
    }
}

pub struct GdbStubHandle {
    pub id: String,
    pub conn_id: String,
    pub addr: String,
}

impl GdbStubHandle {
    pub fn new(id: &str, conn_id: &str, addr: &str) -> Self {
        Self {
            id: id.to_string(),
            conn_id: conn_id.to_string(),
            addr: addr.to_string(),
        }
    }
}
