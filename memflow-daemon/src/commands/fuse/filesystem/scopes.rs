mod connection;
use connection::PhysicalDumpFile;

mod process;
use process::{ProcessInfoFile, ProcessMemoryMaps, ProcessMiniDump};

mod module;
use module::{ModuleDumpFile, ModulePeFolder};

use super::{ChildrenList, FileSystemChildren, FileSystemEntry};
use crate::state::Connection;

use std::sync::{Arc, Mutex};

use memflow::prelude::v1::*;

pub struct ConnectionScope {
    connection: Arc<Mutex<Connection>>,
    name: String,
    children: FileSystemChildren,
}

impl ConnectionScope {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection: Arc::new(Mutex::new(connection)),
            name: std::path::MAIN_SEPARATOR.to_string(),
            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for ConnectionScope {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        Some(self.children.get_or_insert(|| {
            vec![
                Box::new(DriverRootFolder::new(self.connection.clone())),
                Box::new(ProcessRootFolder::new(self.connection.clone())),
                Box::new(PhysicalDumpFile::new(self.connection.clone())),
            ]
        }))
    }
}

/// Describes the root level 'drivers' folder
pub struct DriverRootFolder {
    connection: Arc<Mutex<Connection>>,
    children: FileSystemChildren,
}

impl DriverRootFolder {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self {
            connection,
            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for DriverRootFolder {
    fn name(&self) -> &str {
        "drivers"
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        Some(self.children.get_or_insert(|| {
            let mut result = Vec::new();

            if let Ok(mut connection) = self.connection.lock() {
                match &mut *connection {
                    Connection::Connector(_) => (),
                    Connection::Os(os) => {
                        if let Ok(modules) = os.module_list() {
                            for mi in modules.into_iter() {
                                result.push(Box::new(ModuleFolder::new(
                                    self.connection.clone(),
                                    kernel_proc.proc_info.clone(),
                                    mi,
                                ))
                                    as Box<dyn FileSystemEntry>);
                            }
                        }
                    }
                }
            }

            result
        }))
    }
}

/// Describes the root level 'processes' folder
pub struct ProcessRootFolder {
    connection: Arc<Mutex<Connection>>,
    children: FileSystemChildren,
}

impl ProcessRootFolder {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self {
            connection,
            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for ProcessRootFolder {
    fn name(&self) -> &str {
        "processes"
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        Some(self.children.get_or_insert(|| {
            let mut result = Vec::new();

            if let Ok(mut connection) = self.connection.lock() {
                match &mut *connection {
                    Connection::Connector(_) => (),
                    Connection::Os(os) => {
                        if let Ok(processes) = os.process_info_list() {
                            for pi in processes.into_iter() {
                                result.push(Box::new(ProcessFolder::new(
                                    self.connection.clone(),
                                    pi,
                                ))
                                    as Box<dyn FileSystemEntry>);
                            }
                        }
                    }
                }
            }

            result
        }))
    }
}

// TODO: unify process_info for different osses
pub struct ProcessFolder {
    connection: Arc<Mutex<Connection>>,
    pi: ProcessInfo,

    name: String,
    children: FileSystemChildren,
}
unsafe impl Sync for ProcessFolder {} // TODO: does this hold?

impl ProcessFolder {
    fn new(connection: Arc<Mutex<Connection>>, pi: ProcessInfo) -> Self {
        let name = format!("{}_{}", pi.pid, pi.name);
        Self {
            connection,
            pi,

            name,
            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for ProcessFolder {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        Some(self.children.get_or_insert(|| {
            vec![
                Box::new(ProcessInfoFile::new(&self.pi)),
                Box::new(ProcessMemoryMaps::new(
                    self.connection.clone(),
                    self.pi.clone(),
                )),
                Box::new(ProcessMiniDump::new(
                    self.connection.clone(),
                    self.pi.clone(),
                )),
                Box::new(ModuleRootFolder::new(
                    self.connection.clone(),
                    self.pi.clone(),
                )),
            ]
        }))
    }
}

pub struct ModuleRootFolder {
    connection: Arc<Mutex<Connection>>,
    pi: ProcessInfo,

    children: FileSystemChildren,
}
unsafe impl Sync for ModuleRootFolder {} // TODO: does this hold?

impl ModuleRootFolder {
    fn new(connection: Arc<Mutex<Connection>>, pi: ProcessInfo) -> Self {
        Self {
            connection,
            pi,

            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for ModuleRootFolder {
    fn name(&self) -> &str {
        "modules"
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        // TODO: create files for each driver
        Some(self.children.get_or_insert(|| {
            let mut result = Vec::new();

            if let Ok(mut connection) = self.connection.lock() {
                match &mut *connection {
                    Connection::Connector(_) => (),
                    Connection::Os(os) => {
                        if let Ok(mut process) = os.process_by_info(self.pi.clone()) {
                            if let Ok(modules) = process.module_list() {
                                for mi in modules.into_iter() {
                                    result.push(Box::new(ModuleFolder::new(
                                        self.connection.clone(),
                                        self.pi.clone(),
                                        mi,
                                    ))
                                        as Box<dyn FileSystemEntry>);
                                }
                            }
                        }
                    }
                }
            }

            result
        }))
    }
}

pub struct ModuleFolder {
    connection: Arc<Mutex<Connection>>,
    pi: ProcessInfo,
    mi: ModuleInfo,

    name: String,
    children: FileSystemChildren,
}
unsafe impl Sync for ModuleFolder {} // TODO: does this hold?

// TODO: unify Win32ModuleInfo for different targets
impl ModuleFolder {
    fn new(connection: Arc<Mutex<Connection>>, pi: ProcessInfo, mi: ModuleInfo) -> Self {
        let name = format!("{:x}_{}", mi.base, mi.name);
        Self {
            connection,
            pi,
            mi,

            name,
            children: FileSystemChildren::default(),
        }
    }
}

impl FileSystemEntry for ModuleFolder {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn children(&self) -> Option<ChildrenList> {
        Some(self.children.get_or_insert(|| {
            vec![
                Box::new(ModulePeFolder::new(
                    self.connection.clone(),
                    self.pi.clone(),
                    self.mi.clone(),
                )),
                Box::new(ModuleDumpFile::new(
                    self.connection.clone(),
                    self.pi.clone(),
                    self.mi.clone(),
                )),
            ]
        }))
    }
}
