use crate::error::{Error, Result};
use crate::state::{Connection, STATE};

use memflow::prelude::v1::*;

use crate::memflow_rpc::{
    PhysicalMemoryMetadata, PhysicalMemoryMetadataRequest, PhysicalMemoryMetadataResponse,
    ReadPhysicalMemoryEntryResponse, ReadPhysicalMemoryRequest, ReadPhysicalMemoryResponse,
    WritePhysicalMemoryRequest, WritePhysicalMemoryResponse,
};

pub async fn read(msg: &ReadPhysicalMemoryRequest) -> Result<ReadPhysicalMemoryResponse> {
    let mut state = STATE.lock().await;
    if let Some(conn) = state.connection_mut(&msg.conn_id) {
        // create [PhysicalReadData]
        let mut result_reads = Vec::new();
        let mut read_data = Vec::new();
        for read in msg.reads.iter() {
            result_reads.push(ReadPhysicalMemoryEntryResponse {
                data: vec![0u8; read.len as usize],
            });
        }

        for read in msg.reads.iter().zip(result_reads.iter_mut()) {
            read_data.push(PhysicalReadData(read.0.addr.into(), &mut read.1.data[..]));
        }

        // dispatch to connection
        match &mut conn.connection {
            Connection::Connector(connector) => {
                connector.phys_read_raw_list(&mut read_data.as_mut_slice())?;
            }
            Connection::Os(os) => {
                panic!("os.into_phys_mem() not implemented yet");
            }
        }

        Ok(ReadPhysicalMemoryResponse {
            reads: result_reads,
        })
    } else {
        Err(Error::Connector(format!(
            "no connection with id {} found",
            msg.conn_id
        )))
    }
}

pub async fn write(msg: &WritePhysicalMemoryRequest) -> Result<WritePhysicalMemoryResponse> {
    let mut state = STATE.lock().await;
    if let Some(conn) = state.connection_mut(&msg.conn_id) {
        // create [PhysicalWriteData]
        let mut write_data = Vec::new();
        for write in msg.writes.iter() {
            write_data.push(PhysicalWriteData(write.addr.into(), &write.data.as_slice()));
        }

        // dispatch to connection
        match &mut conn.connection {
            Connection::Connector(connector) => {
                connector.phys_write_raw_list(&write_data.as_slice())?;
            }
            Connection::Os(os) => {
                panic!("os.into_phys_mem() not implemented yet");
            }
        }

        Ok(WritePhysicalMemoryResponse {})
    } else {
        Err(Error::Connector(format!(
            "no connection with id {} found",
            msg.conn_id
        )))
    }
}

pub async fn metadata(
    msg: &PhysicalMemoryMetadataRequest,
) -> Result<PhysicalMemoryMetadataResponse> {
    let mut state = STATE.lock().await;
    if let Some(conn) = state.connection_mut(&msg.conn_id) {
        let metadata = match &mut conn.connection {
            Connection::Connector(connector) => connector.metadata(),
            Connection::Os(os) => {
                panic!("os.into_phys_mem() not implemented yet");
            }
        };

        Ok(PhysicalMemoryMetadataResponse {
            metadata: Some(PhysicalMemoryMetadata {
                size: metadata.size as u64,
                readonly: metadata.readonly,
            }),
        })
    } else {
        Err(Error::Connector(format!(
            "no connection with id {} found",
            msg.conn_id
        )))
    }
}
