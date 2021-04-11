use super::super::{FileSystemEntry, FileSystemFileHandler};
use crate::error::{Error, Result};
use crate::state::Connection;

use std::sync::{Arc, Mutex};

use memflow::prelude::v1::*;

// TODO: block storage?
pub struct PhysicalDumpFile {
    connection: Arc<Mutex<Connection>>,
    metadata: PhysicalMemoryMetadata,
}

impl PhysicalDumpFile {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        let metadata = if let Ok(connection) = connection.lock() {
            match &*connection {
                Connection::Connector(connector) => connector.metadata(),
                Connection::Os(os) => {
                    panic!("os.into_phys_mem() not implemented yet");
                }
            }
        } else {
            PhysicalMemoryMetadata {
                size: 0,
                readonly: false,
            }
        };

        Self {
            connection,
            metadata,
        }
    }
}

impl FileSystemEntry for PhysicalDumpFile {
    fn name(&self) -> &str {
        "dump"
    }

    fn is_leaf(&self) -> bool {
        true
    }

    // TODO: type regularfile,etc
    fn size(&self) -> usize {
        self.metadata.size
    }

    fn is_writable(&self) -> bool {
        !self.metadata.readonly
    }

    fn open(&self) -> Result<Box<dyn FileSystemFileHandler>> {
        if let Ok(connection) = self.connection.lock() {
            Ok(Box::new(PhysicalDumpReader::new(
                connection.clone(),
                self.metadata.clone(),
            )))
        } else {
            Err(Error::Other("unable to lock connection".to_string()))
        }
    }
}

struct PhysicalDumpReader {
    connection: Connection,
    metadata: PhysicalMemoryMetadata,
}

impl PhysicalDumpReader {
    pub fn new(connection: Connection, metadata: PhysicalMemoryMetadata) -> Self {
        Self {
            connection,
            metadata,
        }
    }
}

impl FileSystemFileHandler for PhysicalDumpReader {
    fn read(&mut self, offset: u64, size: u32) -> Result<Vec<u8>> {
        let phys_size = self.metadata.size;
        let real_size = std::cmp::min(size as usize, phys_size - offset as usize);

        match &mut self.connection {
            Connection::Connector(connector) => connector
                .phys_read_raw((offset as u64).into(), real_size)
                .map_err(Error::from),
            Connection::Os(os) => {
                panic!("os.into_phys_mem() not implemented yet");
            }
        }
    }

    fn write(&mut self, offset: u64, data: Vec<u8>) -> Result<usize> {
        match &mut self.connection {
            Connection::Connector(connector) => connector
                .phys_write_raw((offset as u64).into(), &data)
                .map_err(Error::from)
                .map(|_| data.len()),
            Connection::Os(os) => {
                panic!("os.into_phys_mem() not implemented yet");
            }
        }
    }
}
