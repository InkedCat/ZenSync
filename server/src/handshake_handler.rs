use base64::{prelude::BASE64_STANDARD, Engine};
use snow::{HandshakeState, TransportState};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use crate::client_checker::PeerChecker;

#[derive(Error, Debug)]
pub enum ReadInitiatorError {
    #[error("Failed to read packet: {0}")]
    ReadError(#[from] tokio::io::Error),
    #[error("Noise protocol error: {0}")]
    NoiseError(#[from] snow::Error),
}

#[derive(Error, Debug)]
pub enum DoReceiverError {
    #[error("Failed to write packet: {0}")]
    WriteError(#[from] tokio::io::Error),
    #[error("Noise protocol error: {0}")]
    NoiseError(#[from] snow::Error),
}

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("Failed to do the initiator part: {0}")]
    ReadError(#[from] ReadInitiatorError),
    #[error("Failed to do the receiver part: {0}")]
    WriteError(#[from] DoReceiverError),
    #[error("Failed to convert to transport mode: {0}")]
    TransportModeError(#[from] snow::Error),
    #[error("No remote public key")]
    NoRemotePublicKey,
    #[error("Client not in peer list")]
    ClientNotInPeerList,
}

async fn read_initiator(
    buf_reader: &mut BufReader<TcpStream>,
    noise: &mut HandshakeState,
) -> Result<(), ReadInitiatorError> {
    let length = buf_reader.read_u32().await?;
    let mut buffer = vec![0u8; length as usize];

    buf_reader.read_exact(&mut buffer).await?;
    noise.read_message(&mut buffer, &mut [])?;

    Ok(())
}

async fn do_receiver(
    buf_reader: &mut BufReader<TcpStream>,
    noise: &mut HandshakeState,
) -> Result<(), DoReceiverError> {
    let mut write_buffer = vec![0u8; 65535];
    let len = noise.write_message(&[], &mut write_buffer)?;

    buf_reader.write_u32(len as u32).await?;
    buf_reader.write_all(&write_buffer[..len]).await?;

    Ok(())
}

pub async fn handle_handshake(
    buf_reader: &mut BufReader<TcpStream>,
    peer_checker: &PeerChecker,
    mut noise: HandshakeState,
) -> Result<TransportState, HandshakeError> {
    read_initiator(buf_reader, &mut noise).await?;
    let client_public_key = match noise.get_remote_static() {
        Some(key) => key,
        None => return Err(HandshakeError::NoRemotePublicKey),
    };

    let client_public_key = BASE64_STANDARD.encode(client_public_key);
    if !peer_checker.has_client(&client_public_key) {
        return Err(HandshakeError::ClientNotInPeerList);
    }

    do_receiver(buf_reader, &mut noise).await?;

    Ok(noise.into_transport_mode()?)
}
