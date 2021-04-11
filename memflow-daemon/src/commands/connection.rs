use crate::error::{Error, Result};
use crate::state::{Connection, STATE};

use std::convert::TryFrom;

use log::{error, info};
use memflow::prelude::v1::*;

use crate::memflow_rpc::{
    CloseConnectionRequest, CloseConnectionResponse, ConnectionDescription, ListConnectionsRequest,
    ListConnectionsResponse, NewConnectionRequest, NewConnectionResponse,
};

fn create_connection(msg: &NewConnectionRequest) -> Result<Connection> {
    // do a full inventory (re-)scan here
    let inventory = Inventory::scan();

    // try to initialize the os based on the parameters
    let builder = inventory.builder().connector(&msg.connector);

    if !msg.connector_args.is_empty() {
        builder = builder.args(Args::parse(&msg.connector_args)?);
    }

    if msg.os.is_empty() {
        // initialize a connector
        let connector = builder.build()?;
        Ok(Connection::Connector(connector))
    } else {
        // initialize a os
        let os_builder = builder.os(&msg.os);
        if !msg.os_args.is_empty() {
            os_builder = os_builder.args(Args::parse(&msg.os_args)?);
        }
        let os = os_builder.build()?;
        Ok(Connection::Os(os))
    }
}

pub async fn new<'a>(msg: &NewConnectionRequest) -> Result<NewConnectionResponse> {
    match create_connection(msg) {
        Ok(connection) => {
            // TODO: redirect log to client
            // TODO: add cache options

            let mut state = STATE.lock().await;

            let name = format!("{}_{}", msg.connector, msg.os);
            match state.connection_add(
                &name,
                if msg.alias == "" {
                    None
                } else {
                    Some(msg.alias.clone())
                },
                connection,
            ) {
                Ok(id) => {
                    info!(
                        "connection created: {} | {} | {:?} | {} | {:?}",
                        id, msg.connector, msg.connector_args, msg.os, msg.os_args
                    );
                    Ok(NewConnectionResponse { conn_id: id })
                }
                Err(err) => {
                    let err_msg = format!(
                        "could not create connector: {} | {:?} | {} | {:?} ({})",
                        msg.connector, msg.connector_args, msg.os, msg.os_args, err
                    );
                    error!("{}", err_msg);
                    Err(Error::Connector(err_msg))
                }
            }
        }
        Err(err) => {
            let err_msg = format!(
                "could not create connector: {} | {:?} | {} | {:?} ({})",
                msg.connector, msg.connector_args, msg.os, msg.os_args, err
            );
            error!("{}", err_msg);
            Err(Error::Connector(err_msg))
        }
    }
}

pub async fn ls(_msg: &ListConnectionsRequest) -> Result<ListConnectionsResponse> {
    let state = STATE.lock().await;

    info!(
        "listing open connections: {} connections",
        state.connections.len()
    );

    let mut connections = vec![];

    if !state.connections.is_empty() {
        for c in state.connections.iter() {
            connections.push(ConnectionDescription {
                conn_id: c.1.id.clone(),
                name: c.1.name.clone(),
                alias: c
                    .1
                    .alias
                    .as_ref()
                    .map(|a| a.to_string())
                    .unwrap_or_default(),
                refcount: c.1.refcount as u64,
            });
        }
    }

    Ok(ListConnectionsResponse {
        connections: connections,
    })
}

pub async fn rm(msg: &CloseConnectionRequest) -> Result<CloseConnectionResponse> {
    let mut state = STATE.lock().await;

    match state.connection_remove(&msg.conn_id) {
        Ok(_) => {
            info!("connection {} removed", msg.conn_id);
            Ok(CloseConnectionResponse {})
        }
        Err(err) => {
            let err_msg = format!("unable to remove connection {}: {}", msg.conn_id, err);
            error!("{}", err_msg);
            Err(Error::Connector(err_msg))
        }
    }
}
