mod cli;
mod client_checker;
mod config;
mod handshake_handler;

use cli::Args;
use config::ServerConfig;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use commons::{file_manager, packeter};
use nix::unistd::Uid;

use anyhow::{bail, Context};
use base64::{prelude::BASE64_STANDARD, Engine};
use client_checker::PeerChecker;
use protos::{
    deserialize_request, deserialize_request_add, deserialize_request_get,
    deserialize_request_move, deserialize_request_remove, deserialize_request_sync, RequestType,
};
use tokio::{
    fs,
    io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
};

const MAX_CONCURRENT_CONNECTIONS: usize = 10;

struct Virtualizer {
    user_path: PathBuf,
}

impl Virtualizer {
    fn new(user_path: PathBuf) -> Self {
        Self { user_path }
    }

    fn v_path(&self, path: &PathBuf) -> io::Result<PathBuf> {
        if path.starts_with("/") {
            Ok(self.user_path.join(path.strip_prefix("/").unwrap()))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Path must start with /"),
            ))
        }
    }

    fn uv_path(&self, path: &PathBuf) -> io::Result<PathBuf> {
        if path.starts_with(&self.user_path) {
            let relative_path = path.strip_prefix(&self.user_path).unwrap();
            Ok(Path::new("/").join(relative_path))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Path must start with {}", &self.user_path.display()),
            ))
        }
    }
}

async fn handle_add_request(
    handler: &mut packeter::Handler,
    request: protos::RequestAdd,
    virtualizer: &Virtualizer,
) -> anyhow::Result<()> {
    let request_file = if let Some(file) = request.file {
        file
    } else {
        return Err(anyhow::anyhow!("No file in request"));
    };

    let true_path = PathBuf::from(request_file.path);
    let virtual_path = virtualizer.v_path(&true_path)?;

    if let Some(parent) = virtual_path.parent() {
        if !fs::try_exists(&parent).await? {
            fs::create_dir_all(&parent).await?;
        }
    }

    let (mut temp_file, temp_path) = file_manager::open_temporary_file(&virtual_path).await?;

    let mut remaining_bytes = request_file.size;
    loop {
        if remaining_bytes == 0 {
            break;
        }

        let payload_chunk = match handler.read_packet().await {
            Ok(chunk) => chunk,
            Err(e) => match e {
                packeter::ReadPacketError::ReadError(_) => {
                    continue;
                }
                _ => break,
            },
        };

        println!("Received chunk of {} bytes", payload_chunk.len());

        temp_file.write_all(&payload_chunk).await?;
        remaining_bytes -= payload_chunk.len() as u64;
    }

    let std_temp_file = temp_file.into_std().await;
    file_manager::close_temporary_file(
        std_temp_file,
        &temp_path,
        &virtual_path,
        request_file.last_modified,
        request_file.file_permissions,
        request_file.file_owner,
        request_file.file_group,
    )
    .await?;

    Ok(())
}

async fn handle_move(request: &protos::RequestMove) -> anyhow::Result<()> {
    for file in &request.files {
        let old_path = Path::new(&file.old_path);
        if !fs::try_exists(&old_path).await? {
            continue;
        }

        let new_path = Path::new(&file.new_path);
        fs::rename(&old_path, &new_path).await?;
    }

    Ok(())
}

async fn handle_delete(
    request: &protos::RequestRemove,
    virtualizer: &Virtualizer,
) -> anyhow::Result<()> {
    for file in &request.files {
        let true_path = PathBuf::from(&file.path);
        let virtual_path = virtualizer.v_path(&true_path)?;
        if !fs::try_exists(&virtual_path).await? {
            continue;
        }

        fs::remove_file(virtual_path).await?;
    }

    Ok(())
}

async fn visit_dirs(
    virtualizer: &Virtualizer,
    dir: &PathBuf,
    file_stats: &mut protos::File,
) -> anyhow::Result<()> {
    if dir.is_dir() {
        let mut dir = fs::read_dir(&dir).await?;
        while let Ok(entry) = dir.next_entry().await {
            let entry = match entry {
                Some(entry) => entry,
                None => break,
            };

            let virtual_path = entry.path();
            let metadata = fs::metadata(&virtual_path).await?;
            let true_path = virtualizer.uv_path(&virtual_path)?;
            let mut entry_stats =
                file_manager::create_from_metadata(&true_path, Some(&virtual_path), &metadata)
                    .await?;

            if virtual_path.is_dir() {
                Box::pin(visit_dirs(virtualizer, &virtual_path, &mut entry_stats)).await?;
            }

            file_stats.childrens.push(entry_stats);
        }
    }

    Ok(())
}

async fn handle_get(
    handler: &mut packeter::Handler,
    virtualizer: &Virtualizer,
    request: &protos::RequestGet,
) -> anyhow::Result<()> {
    let mut response = protos::ResponseGet::default();
    for file in &request.files {
        let true_path = PathBuf::from(&file.path);
        let virtual_path = virtualizer.v_path(&true_path)?;

        if !fs::try_exists(&virtual_path).await? {
            continue;
        }

        let metadata = fs::metadata(&virtual_path).await?;
        let mut file_stats =
            file_manager::create_from_metadata(&true_path, Some(&virtual_path), &metadata).await?;
        visit_dirs(virtualizer, &virtual_path, &mut file_stats).await?;
        response.files.push(file_stats);
    }

    let response = protos::serialize_response_get(response);
    handler.write_packet(response.as_slice()).await?;

    Ok(())
}

async fn handle_sync(
    handler: &mut packeter::Handler,
    request: &protos::RequestSync,
    virtualizer: &Virtualizer,
) -> anyhow::Result<()> {
    let file_to_sync = if let Some(file) = &request.file {
        file
    } else {
        bail!("No file in request");
    };

    let file_path = PathBuf::from(&file_to_sync.path);
    let virtual_path = virtualizer.v_path(&file_path)?;
    let file = tokio::fs::File::open(&virtual_path).await?;

    let metadata = file.metadata().await?;
    let file_stats =
        file_manager::create_from_metadata(&file_path, Some(&virtual_path), &metadata).await?;
    let size = file_stats.size.clone();
    let request_sync = protos::create_response_sync(file_stats, size);

    let request = protos::serialize_response_sync(request_sync);
    handler.write_packet(&request).await?;

    let mut file_reader = BufReader::new(file);

    let mut read_buf = [0u8; 66536];
    loop {
        let n = file_reader.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }

        handler.write_packet(&read_buf[..n]).await?;
    }

    Ok(())
}

async fn handle_client(
    stream: TcpStream,
    noise: snow::HandshakeState,
    save_path: String,
    peer_checker: PeerChecker,
) -> anyhow::Result<()> {
    let mut buf_reader = BufReader::new(stream);
    let noise = handshake_handler::handle_handshake(&mut buf_reader, &peer_checker, noise).await?;

    let client_key = match noise.get_remote_static() {
        Some(key) => key,
        None => return Err(anyhow::anyhow!("No remote public key")),
    };
    let public_key = BASE64_STANDARD.encode(client_key);
    let user = peer_checker.get_peer(&public_key).unwrap();
    println!("Client authenticated as: {}", user.username);

    let user_path = Path::new(&save_path).join(&user.username);
    if !fs::try_exists(&user_path).await? {
        fs::create_dir(&user_path).await?;
    }

    let mut handler = packeter::Handler::new(buf_reader, noise);
    let virtualizer = Virtualizer::new(user_path);

    while let Ok(msg) = handler.read_packet().await {
        let request = deserialize_request(&msg)?;

        match RequestType::try_from(request.request_type) {
            Ok(RequestType::Add) => {
                let request = deserialize_request_add(&msg)?;

                println!("Received add request: {:?}", request);

                handle_add_request(&mut handler, request, &virtualizer).await?;

                println!("File received");
            }
            Ok(RequestType::Move) => {
                let request = deserialize_request_move(&msg)?;

                println!("Received move request: {:?}", request);

                handle_move(&request).await?;
            }
            Ok(RequestType::Remove) => {
                let request = deserialize_request_remove(&msg)?;

                println!("Received remove request: {:?}", request);

                handle_delete(&request, &virtualizer).await?;
            }
            Ok(RequestType::Get) => {
                let request = deserialize_request_get(&msg)?;

                println!("Received get request: {:?}", request);

                handle_get(&mut handler, &virtualizer, &request).await?;
            }
            Ok(RequestType::Sync) => {
                let request = deserialize_request_sync(&msg)?;

                println!("Received sync request: {:?}", request);

                handle_sync(&mut handler, &request, &virtualizer).await?;
            }
            Err(_) => {
                println!("Received unknown request type: {:?}", request.request_type);
            }
        }
    }

    Ok(())
}

async fn init(_args: &Args, config: &ServerConfig) -> anyhow::Result<()> {
    let save_path = Path::new(&config.folder);

    if !fs::try_exists(&save_path).await? {
        fs::create_dir_all(&save_path).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !cfg!(target_arch = "x86_64") {
        eprintln!("This program is only supported on x86_64");
        return Ok(());
    }

    if !cfg!(target_os = "linux") {
        eprintln!("This program is only supported on Linux");
        return Ok(());
    }

    if !Uid::effective().is_root() {
        eprintln!("This program must be run as root");
        return Ok(());
    }

    let args = cli::get_args();

    let config = config::get_config(&args.config_path)
        .with_context(|| format!("Failed to parse config file at {}", &args.config_path))?;

    init(&args, &config.server).await?;

    let addr = format!("{}:{}", &args.addr, &config.server.port);
    let listener: TcpListener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", &addr))?;
    println!("Listening on {}", &addr);

    let private_key = BASE64_STANDARD.decode(&config.server.private_key)?;

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                println!("New client connected");
                let permit = semaphore.clone().acquire_owned().await?;

                let peer_checker = PeerChecker::new(config.peer.clone());

                // Unwrap for now as it is unrecoverable if it happens
                let noise = snow::Builder::new(commons::NOISE_PARAMS.clone())
                    .local_private_key(&private_key)
                    .build_responder()
                    .unwrap();
                let save_folder = config.server.folder.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, noise, save_folder, peer_checker).await {
                        eprintln!("Error handling client: {}", e);
                    }

                    drop(permit);
                });
            }
            Err(err) => {
                eprintln!("Error accepting client: {}", err);
                break;
            }
        }
    }

    Ok(())
}
