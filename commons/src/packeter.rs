use snow::TransportState;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[derive(Error, Debug)]
pub enum ReadPacketError {
    #[error("Failed to read packet: {0}")]
    ReadError(#[from] tokio::io::Error),
    #[error("End of file reached")]
    EOF,
    #[error("Noise protocol error: {0}")]
    NoiseError(#[from] snow::Error),
}

#[derive(Error, Debug)]
pub enum WritePacketError {
    #[error("Failed to write packet: {0}")]
    WriteError(#[from] tokio::io::Error),
    #[error("Noise protocol error: {0}")]
    NoiseError(#[from] snow::Error),
}

pub struct Handler {
    buf_reader: BufReader<TcpStream>,
    noise: TransportState,
}

impl Handler {
    pub fn new(buf_reader: BufReader<TcpStream>, noise: TransportState) -> Self {
        Self { buf_reader, noise }
    }

    pub async fn read_packet(&mut self) -> Result<Vec<u8>, ReadPacketError> {
        let mut read_buffer = vec![0u8; 65535];
        let mut decrypt_buffer = vec![0u8; 65535];

        let n = self.buf_reader.read_u32().await.map_err(|e| {
            if e.kind() == tokio::io::ErrorKind::UnexpectedEof {
                ReadPacketError::EOF
            } else {
                ReadPacketError::ReadError(e)
            }
        })?;

        self.buf_reader
            .read_exact(&mut read_buffer[..n as usize])
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    ReadPacketError::EOF
                } else {
                    ReadPacketError::ReadError(e)
                }
            })?;
        let read = self
            .noise
            .read_message(&mut read_buffer[..n as usize], &mut decrypt_buffer)?;

        Ok(decrypt_buffer[..read].to_vec())
    }

    pub async fn write_packet(&mut self, buf: &[u8]) -> Result<(), WritePacketError> {
        let mut encrypted_buffer = vec![0u8; 65535];
        let len = self.noise.write_message(&buf, &mut encrypted_buffer)?;

        self.buf_reader.write_u32(len as u32).await?;
        self.buf_reader.write_all(&encrypted_buffer[..len]).await?;

        Ok(())
    }
}
