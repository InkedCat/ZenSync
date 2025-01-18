mod config;
mod handshake_handler;

use std::{collections::VecDeque, path::PathBuf};

use anyhow::Context;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use dirs_next::config_dir;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};

use commons::{file_manager, packeter};
use protos::{
    create_file_get, create_file_sync, File, FileGet, FileType, ResponseGet, ResponseSync,
};

async fn send_file(handler: &mut packeter::Handler, file_path: &PathBuf) -> anyhow::Result<()> {
    let file = tokio::fs::File::open(&file_path).await?;
    let metadata = file.metadata().await?;
    let file_stats =
        file_manager::create_from_metadata(&file_path, Some(&file_path), &metadata).await?;
    let size = file_stats.size.clone();
    let request_add = protos::create_request_add(file_stats, size);

    let request = protos::serialize_request_add(request_add);
    handler.write_packet(&request).await?;

    let mut file_reader = BufReader::new(file);

    let mut read_buf = [0u8; 32768];
    loop {
        let n = file_reader.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }

        handler.write_packet(&read_buf[..n]).await?;
    }

    Ok(())
}
// TODO: SPACES
async fn send_folder(handler: &mut packeter::Handler, folder: &PathBuf) -> anyhow::Result<()> {
    let mut dir = fs::read_dir(folder).await?;
    while let Ok(entry) = dir.next_entry().await {
        let entry = match entry {
            Some(entry) => entry,
            None => break,
        };

        let path = entry.path();
        if path.is_dir() {
            Box::pin(send_folder(handler, &path)).await?;
        } else {
            send_file(handler, &path).await?;
        }
    }

    Ok(())
}

async fn receive_file(
    handler: &mut packeter::Handler,
    response: ResponseSync,
) -> anyhow::Result<()> {
    let response_file = if let Some(file) = response.file {
        file
    } else {
        return Err(anyhow::anyhow!("No file in request"));
    };

    let file_path = PathBuf::from(response_file.path);

    if let Some(parent) = file_path.parent() {
        if !fs::try_exists(&parent).await? {
            fs::create_dir_all(&parent).await?;
        }
    } else {
        return Err(anyhow::anyhow!("No parent path"));
    }

    let (mut temp_file, temp_path) = file_manager::open_temporary_file(&file_path).await?;

    let mut remaining_bytes = response_file.size;
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
        &file_path,
        response_file.last_modified,
        response_file.file_permissions,
        response_file.file_owner,
        response_file.file_group,
    )
    .await?;

    Ok(())
}

async fn send_request_get(
    handler: &mut packeter::Handler,
    files: Vec<FileGet>,
) -> anyhow::Result<()> {
    let request_get = protos::create_request_get(files);
    let request = protos::serialize_request_get(request_get);
    handler.write_packet(&request).await?;

    Ok(())
}

async fn receive_response_get(handler: &mut packeter::Handler) -> anyhow::Result<ResponseGet> {
    let response = handler.read_packet().await?;
    let response = protos::deserialize_response_get(&response)?;

    Ok(response)
}

async fn send_request_sync(handler: &mut packeter::Handler, path: String) -> anyhow::Result<()> {
    let file_sync = create_file_sync(path);
    let request_sync = protos::create_request_sync(file_sync);
    let request = protos::serialize_request_sync(request_sync);
    handler.write_packet(&request).await?;

    Ok(())
}

async fn process_response_get(
    mut handler: &mut packeter::Handler,
    files: Vec<File>,
) -> anyhow::Result<()> {
    for file in files {
        if file.file_type == FileType::Directory as i32 {
            Box::pin(process_response_get(handler, file.childrens)).await?;
        } else {
            send_request_sync(&mut handler, file.path).await?;

            receive_response_sync(&mut handler).await?;
        }
    }

    Ok(())
}

async fn send_folder_request_sync(
    handler: &mut packeter::Handler,
    folder: &PathBuf,
) -> anyhow::Result<()> {
    let file_get = create_file_get(folder.to_str().unwrap().to_string());
    let mut files = Vec::new();
    files.push(file_get);
    send_request_get(handler, files).await?;

    let response = receive_response_get(handler).await?;

    process_response_get(handler, response.files).await?;

    Ok(())
}

async fn receive_response_sync(handler: &mut packeter::Handler) -> anyhow::Result<()> {
    let response = handler.read_packet().await?;
    let response = protos::deserialize_response_sync(&response)?;

    receive_file(handler, response).await?;

    Ok(())
}

pub async fn handle(
    rx: &mut mpsc::Receiver<String>,
    tx_get: &mut mpsc::Sender<String>,
    tx_sync: &mut mpsc::Sender<String>,
    tx_add: &mut mpsc::Sender<String>,
) -> anyhow::Result<()> {
    let config_path = if let Some(config_path) = config_dir() {
        config_path
    } else {
        return Err(anyhow::anyhow!("Failed to get config path"));
    };

    let config = config::get_config(&config_path.join("zen-sync").join("config.toml"))
        .with_context(|| format!("Failed to parse config file at {}", "config.toml"))?;

    let home_config = config.peer.get("home").unwrap();

    let local_private_key = BASE64_STANDARD.decode(&config.client.private_key)?;
    let remote_public_key = BASE64_STANDARD.decode(&home_config.public_key)?;

    let stream: TcpStream = TcpStream::connect("localhost:8080")
        .await
        .with_context(|| format!("Failed to connect to {}", "localhost:8080"))?;

    let mut buf_reader = BufReader::new(stream);

    let noise = snow::Builder::new(commons::NOISE_PARAMS.clone())
        .local_private_key(local_private_key.as_slice())
        .remote_public_key(remote_public_key.as_slice())
        .build_initiator()?;

    let noise = handshake_handler::handle_handshake(&mut buf_reader, noise).await?;

    let mut handler = packeter::Handler::new(buf_reader, noise);

    while let Some(message) = rx.recv().await {
        let mut parts = message.split(" ").collect::<VecDeque<&str>>();
        match parts.pop_front().unwrap() {
            "sync" => {
                println!("Sending sync request for {:?}", parts);
                let pop = parts.pop_front().unwrap();
                let file = String::from(pop);
                send_request_sync(&mut handler, file).await?;

                receive_response_sync(&mut handler).await?;
                tx_sync.send(String::from(pop)).await?;
            }
            "sync_folder" => {
                println!("Sending sync_folder request for {:?}", parts);
                let pop = parts.pop_front().unwrap();
                let folder = PathBuf::from(pop);

                send_folder_request_sync(&mut handler, &folder).await?;
                tx_sync.send(String::from(pop)).await?;
            }
            "get_all" => {
                println!("Sending get request for {:?}", parts);
                let files = parts
                    .iter()
                    .map(|path| create_file_get(String::from(*path)))
                    .collect::<Vec<FileGet>>();

                println!("Sending request get for {:?}", files);

                send_request_get(&mut handler, files).await?;
                println!("{:?}", receive_response_get(&mut handler).await?);
            }
            "add" => {
                println!("Sending add request for {:?}", parts);
                let pop = parts.pop_front().unwrap();
                let file = PathBuf::from(pop);
                send_file(&mut handler, &file).await?;
                tx_add.send(String::from(pop)).await?;
            }
            "add_folder" => {
                println!("Sending add_file request for {:?}", parts);
                let pop = parts.pop_front().unwrap();
                let folder = PathBuf::from(pop);

                send_folder(&mut handler, &folder).await?;
                tx_add.send(String::from(pop)).await?;
            }
            "remove" => {
                println!("Sending remove request for {:?}", parts);
                let pop = parts.pop_front().unwrap();
                let file_remove = protos::create_file_remove(String::from(pop));
                let files = Vec::from([file_remove]);
                let request_remove = protos::create_request_remove(files);
                let request = protos::serialize_request_remove(request_remove);
                handler.write_packet(&request).await?;
            }
            _ => {
                println!("Unknown message: {}", message);
            }
        }
    }

    Ok(())
}

// pub struct ZsyncClient {
//     handler: packeter::Handler,
// }

// impl ZsyncClient {
//     pub async fn new() -> anyhow::Result<Self> {
//         let config = config::get_config("config.toml")
//             .with_context(|| format!("Failed to parse config file at {}", "config.toml"))?;

//         let home_config = config.peer.get("home").unwrap();

//         let local_private_key = BASE64_STANDARD.decode(&config.client.private_key)?;
//         let remote_public_key = BASE64_STANDARD.decode(&home_config.public_key)?;

//         let stream: TcpStream = TcpStream::connect("localhost:8080")
//             .await
//             .with_context(|| format!("Failed to connect to {}", "localhost:8080"))?;

//         let mut buf_reader = BufReader::new(stream);

//         let noise = snow::Builder::new(commons::NOISE_PARAMS.clone())
//             .local_private_key(local_private_key.as_slice())
//             .remote_public_key(remote_public_key.as_slice())
//             .build_initiator()?;

//         let noise = handshake_handler::handle_handshake(&mut buf_reader, noise).await?;

//         let handler = packeter::Handler::new(buf_reader, noise);

//         Ok(Self { handler })
//     }

//     pub async fn send_file(&mut self, file_path: &PathBuf) -> anyhow::Result<()> {
//         send_file(&mut self.handler, file_path).await
//     }

//     pub async fn send_request_get(&mut self, files: Vec<FileGet>) -> anyhow::Result<()> {
//         send_request_get(&mut self.handler, files).await
//     }
// }

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let config = config::get_config("config.toml")
//         .with_context(|| format!("Failed to parse config file at {}", "config.toml"))?;

//     let home_config = config.peer.get("home").unwrap();

//     let local_private_key = BASE64_STANDARD.decode(&config.client.private_key)?;
//     let remote_public_key = BASE64_STANDARD.decode(&home_config.public_key)?;

//     let stream: TcpStream = TcpStream::connect("localhost:8080")
//         .await
//         .with_context(|| format!("Failed to connect to {}", "localhost:8080"))?;

//     let mut buf_reader = BufReader::new(stream);

//     let noise = snow::Builder::new(commons::NOISE_PARAMS.clone())
//         .local_private_key(local_private_key.as_slice())
//         .remote_public_key(remote_public_key.as_slice())
//         .build_initiator()?;

//     let noise = handshake_handler::handle_handshake(&mut buf_reader, noise).await?;

//     let mut handler = packeter::Handler::new(buf_reader, noise);

//     send_file(&mut handler, &PathBuf::from("./kilian")).await?;

//     let files = vec![create_file_get(String::from(
//         "/home/merlanda/Github/zen-sync/client/kilian",
//     ))];
//     send_request_get(&mut handler, files).await?;

//     while let Ok(msg) = handler.read_packet().await {
//         let response = deserialize_response(&msg)?;

//         println!("Received response: {:?}", response);

//         match ResponseType::try_from(response.response_type) {
//             Ok(ResponseType::Get) => {
//                 let response = deserialize_response_get(&msg)?;
//                 println!("Received add request: {:?}", response);
//             }
//             Ok(ResponseType::Sync) => {
//                 println!("Received sync request");
//             }
//             Err(_) => {
//                 println!(
//                     "Received unknown response type: {:?}",
//                     response.response_type
//                 );
//             }
//         }
//     }

//     Ok(())
// }
