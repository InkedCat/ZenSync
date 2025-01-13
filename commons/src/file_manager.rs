use std::{
    fs::{Metadata, Permissions},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use anyhow::bail;
use rand::Rng;
use thiserror::Error;
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, BufReader},
};

#[derive(Error, Debug)]
pub enum CreateTemporaryFileError {
    #[error("Failed to create temporary file: {0}")]
    CreateError(#[from] std::io::Error),
    #[error("Parent path does not exist")]
    ParentPathDoesNotExist,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Invalid file name")]
    InvalidFileName,
}

pub fn random_suffix() -> String {
    let mut rng = rand::thread_rng();
    let characters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..characters.len());
            characters.chars().nth(idx).unwrap()
        })
        .collect()
}

pub fn file_type_convert(file_type: std::fs::FileType) -> anyhow::Result<protos::FileType> {
    if file_type.is_dir() {
        Ok(protos::FileType::Directory)
    } else if file_type.is_file() {
        Ok(protos::FileType::File)
    } else {
        bail!("Unsupported file type");
    }
}

async fn hash_file(file_path: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let mut hasher = blake3::Hasher::new();
    let file = File::open(file_path).await?;
    let mut file_reader = BufReader::new(file);

    let mut buf = vec![0u8; 524288];
    loop {
        let bytes_read = file_reader.read(&mut buf).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buf[..bytes_read]);
    }

    Ok(hasher.finalize().as_bytes().to_vec())
}

pub async fn create_from_metadata(
    file_path: &PathBuf,
    hash_path: Option<&PathBuf>,
    metadata: &Metadata,
) -> anyhow::Result<protos::File> {
    let file_type = file_type_convert(metadata.file_type())?;
    let path = String::from(file_path.to_str().unwrap());
    let len = metadata.len();

    let mut hash = None;
    if !metadata.file_type().is_dir() {
        if let Some(hash_path) = hash_path {
            hash = Some(hash_file(hash_path).await?);
        }
    }

    let file_owner = metadata.uid();
    let file_group = metadata.gid();
    let file_permissions = metadata.permissions().mode();
    let last_modified = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let file_add = protos::create_file(
        file_type,
        path,
        len,
        hash,
        file_owner,
        file_group,
        file_permissions,
        last_modified,
    );

    Ok(file_add)
}

pub async fn open_temporary_file(
    path: &PathBuf,
) -> Result<(tokio::fs::File, PathBuf), CreateTemporaryFileError> {
    let path = path.clone();
    let parent_path = if let Some(path) = path.parent() {
        if !path.exists() {
            return Err(CreateTemporaryFileError::ParentPathDoesNotExist);
        }

        path
    } else {
        return Err(CreateTemporaryFileError::InvalidPath);
    };

    let temp_name = if let Some(name) = path.file_name() {
        format!("{}.{}", name.to_str().unwrap(), random_suffix())
    } else {
        return Err(CreateTemporaryFileError::InvalidFileName);
    };

    let temp_path = parent_path.join(temp_name);
    let temp_file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(&temp_path)
        .await?;

    Ok((temp_file, temp_path))
}

pub async fn close_temporary_file(
    temp_file: std::fs::File,
    temp_path: &PathBuf,
    final_path: &PathBuf,
    last_modified: u64,
    file_permissions: u32,
    file_owner: u32,
    file_group: u32,
) -> Result<(), std::io::Error> {
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(last_modified);
    let permissions = Permissions::from_mode(file_permissions);

    let clone = temp_path.clone();
    tokio::task::spawn_blocking(move || {
        let _ = temp_file.set_modified(modified);
        let _ = temp_file.set_permissions(permissions);
        std::os::unix::fs::chown(clone, Some(file_owner), Some(file_group))
    });

    fs::rename(temp_path, final_path).await?;

    Ok(())
}
