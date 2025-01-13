mod requests {
    include!(concat!(env!("OUT_DIR"), "/requests.rs"));
}
mod file {
    include!(concat!(env!("OUT_DIR"), "/file.rs"));
}
mod responses {
    include!(concat!(env!("OUT_DIR"), "/responses.rs"));
}

pub use file::{File, FileGet, FileSync, FileType};
use prost::{DecodeError, Message};
pub use requests::{
    Request, RequestAdd, RequestGet, RequestMove, RequestRemove, RequestSync, RequestType,
};
pub use responses::{ResponseGet, ResponseSync, ResponseType};

// File Functions
pub fn create_file(
    file_type: file::FileType,
    path: String,
    size: u64,
    hash: Option<Vec<u8>>,
    file_owner: u32,
    file_group: u32,
    file_permissions: u32,
    last_modified: u64,
) -> file::File {
    let mut file = file::File::default();
    file.file_type = file_type as i32;
    file.path = path;
    file.size = size;
    file.hash = hash;
    file.file_owner = file_owner;
    file.file_group = file_group;
    file.file_permissions = file_permissions;
    file.last_modified = last_modified;
    file.childrens = Vec::new();
    file
}

pub fn create_file_move(old_path: String, new_path: String) -> file::FileMove {
    let mut file = file::FileMove::default();
    file.old_path = old_path;
    file.new_path = new_path;
    file
}

pub fn create_file_remove(path: String) -> file::FileRemove {
    let mut file = file::FileRemove::default();
    file.path = path;
    file
}

pub fn create_file_get(path: String) -> FileGet {
    let mut file = file::FileGet::default();
    file.path = path;
    file
}

pub fn create_file_sync(path: String) -> file::FileSync {
    let mut file = file::FileSync::default();
    file.path = path;
    file
}

// Requests Functions
pub fn deserialize_request(buf: &[u8]) -> Result<requests::Request, DecodeError> {
    requests::Request::decode(buf)
}

pub fn create_request_add(file: file::File, data_len: u64) -> requests::RequestAdd {
    let mut request = requests::RequestAdd::default();
    request.request_type = RequestType::Add as i32;
    request.file = Some(file);
    request.payoad_size = data_len;
    request
}

pub fn serialize_request_add(request: requests::RequestAdd) -> Vec<u8> {
    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_request_add(buf: &[u8]) -> Result<requests::RequestAdd, DecodeError> {
    requests::RequestAdd::decode(buf)
}

pub fn create_request_move(files: Vec<file::FileMove>) -> requests::RequestMove {
    let mut request = requests::RequestMove::default();
    request.request_type = RequestType::Move as i32;
    request.files = files;
    request
}

pub fn serialize_request_move(request: requests::RequestMove) -> Vec<u8> {
    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_request_move(buf: &[u8]) -> Result<requests::RequestMove, DecodeError> {
    requests::RequestMove::decode(buf)
}

pub fn create_request_remove(files: Vec<file::FileRemove>) -> requests::RequestRemove {
    let mut request = requests::RequestRemove::default();
    request.request_type = RequestType::Remove as i32;
    request.files = files;
    request
}

pub fn serialize_request_remove(request: requests::RequestRemove) -> Vec<u8> {
    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_request_remove(buf: &[u8]) -> Result<requests::RequestRemove, DecodeError> {
    requests::RequestRemove::decode(buf)
}

pub fn create_request_get(files: Vec<FileGet>) -> requests::RequestGet {
    let mut request = requests::RequestGet::default();
    request.request_type = RequestType::Get as i32;
    request.files = files;
    request
}

pub fn serialize_request_get(request: requests::RequestGet) -> Vec<u8> {
    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_request_get(buf: &[u8]) -> Result<requests::RequestGet, DecodeError> {
    requests::RequestGet::decode(buf)
}

pub fn create_request_sync(file: file::FileSync) -> requests::RequestSync {
    let mut request = requests::RequestSync::default();
    request.request_type = RequestType::Sync as i32;
    request.file = Some(file);
    request
}

pub fn serialize_request_sync(request: requests::RequestSync) -> Vec<u8> {
    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_request_sync(buf: &[u8]) -> Result<requests::RequestSync, DecodeError> {
    requests::RequestSync::decode(buf)
}

// Responses Functions
pub fn deserialize_response(buf: &[u8]) -> Result<responses::Response, DecodeError> {
    responses::Response::decode(buf)
}

pub fn create_response_get(files: Vec<file::File>) -> responses::ResponseGet {
    let mut response = responses::ResponseGet::default();
    response.response_type = ResponseType::Get as i32;
    response.files = files;
    response
}

pub fn serialize_response_get(response: responses::ResponseGet) -> Vec<u8> {
    let mut buf = Vec::new();
    response.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_response_get(buf: &[u8]) -> Result<responses::ResponseGet, DecodeError> {
    responses::ResponseGet::decode(buf)
}

pub fn create_response_sync(file: file::File, data_len: u64) -> responses::ResponseSync {
    let mut response = responses::ResponseSync::default();
    response.response_type = ResponseType::Sync as i32;
    response.file = Some(file);
    response.payload_size = data_len;
    response
}

pub fn serialize_response_sync(response: responses::ResponseSync) -> Vec<u8> {
    let mut buf = Vec::new();
    response.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_response_sync(buf: &[u8]) -> Result<responses::ResponseSync, DecodeError> {
    responses::ResponseSync::decode(buf)
}
