use std::env;
use std::io;
use std::path::Path;
use std::fs::{self, Metadata, create_dir_all};
use std::time::SystemTime;
use crate::utils::filesystem;
use crate::utils::messages;
use crate::utils::{integrity, conf_file};
use serde::{Deserialize, Serialize};
use filesystem::{read_data, write_data, create_file, check_file_exists};
use serde_json::Result;
use dirs::home_dir;
use std::path::PathBuf;


pub struct FolderInfo {
    pub folders_count: usize,
    pub files_count: usize,
    pub size: u64
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FileType {
    File,
    Folder,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FolderData {
    pub path: String,
    pub is_sync: bool,
    pub file_type: FileType,
    pub last_modified: SystemTime,
    pub size: u64,
}

impl FolderData {
    /// Constructs a new `FolderData`, computing the size and metadata for the given path.
    pub fn new(path: String, is_sync: bool, file_type: FileType) -> io::Result<Self> {
        let metadata = fs::metadata(&path)?; // Unwrap the Result here
        let last_modified = metadata.modified()?; // Access the last modification time

        let size = if let FileType::Folder = file_type {
            FolderData::compute_folder_size(Path::new(&path))? // Compute folder size
        } else {
            metadata.len() // Get file size
        };

        Ok(FolderData {
            path,
            is_sync,
            file_type,
            last_modified,
            size,
        })
    }
    /// Computes the total size of a folder by summing the sizes of all files and subfolders.
    fn compute_folder_size(path: &Path) -> io::Result<u64> {
        let mut total_size = 0;

        if path.is_dir() {
            for entry_result in fs::read_dir(path)? {
                let entry = entry_result?; // Handle the Result from read_dir
                let metadata = entry.metadata()?; // Get metadata of the entry

                if metadata.is_dir() {
                    // If it's a directory, recursively compute its size
                    total_size += FolderData::compute_folder_size(&entry.path())?;
                } else {
                    // If it's a file, add its size
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    // Extracts the name of the file or folder from the path.
    pub fn get_name(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                if name.len() > 20 {
                    let mut truncated = name[..15].to_string();
                    truncated.push_str("...");
                    truncated
                } else {
                    name.to_string()
                }
            })
            .unwrap_or_else(|| "".to_string())
    }

    pub fn get_folder_info(&self) -> Option<FolderInfo> {
        let path = Path::new(&self.path);

        if !path.exists() {
            return None; // Return None if the path does not exist
        }

        if path.is_file() {
            // If it's a file, 0 folders and 1 file
            return Some(FolderInfo {
                folders_count: 0,
                files_count: 1,
                size: FolderData::compute_folder_size(path).unwrap_or(0),
            });
        }

        if path.is_dir() {
            // Recursively count folders and files
            fn count_entries_recursive(path: &Path) -> (usize, usize) {
                let mut folders_count = 0;
                let mut files_count = 0;

                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            folders_count += 1;
                            let (sub_folders, sub_files) = count_entries_recursive(&entry_path);
                            folders_count += sub_folders;
                            files_count += sub_files;
                        } else if entry_path.is_file() {
                            files_count += 1;
                        }
                    }
                }
                (folders_count, files_count)
            }

            let (folders_count, files_count) = count_entries_recursive(path);

            return Some(FolderInfo {
                folders_count,
                files_count,
                size: FolderData::compute_folder_size(path).unwrap_or(0),
            });
        }

        None // In case the path is neither a file nor a folder
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Frequency {
    pub frequency: String 
}

impl Frequency {
    pub const valid_frequencies: [&str; 12] = ["Aucune","1min","5min","30min","1hr","2hr","4hr","6hr","12hr","24hr","2days","7days"];
    pub fn isValid(&self) -> bool {
        return Self::valid_frequencies.contains(&self.frequency.as_str());
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigurationData {
    pub folders: Vec<FolderData>,
    pub frequency: Frequency, 
    pub username: String,
    pub pub_key: String,
    pub private_key: String
}

impl ConfigurationData {
    fn clone(&self) -> ConfigurationData {
        return ConfigurationData {
            folders: self.folders.clone(),
            frequency: Frequency {frequency:String::from("")},
            username: self.username.clone(),
            pub_key:String::from(""),
            private_key:String::from("")
        };
    }

    // Permet de récupérer les données de la configuration actuelle de l'application.
    pub fn get_data(&mut self){
        let conf_path = conf_file::get_conf_path();
        let file_path = Path::new(&conf_path);

        if !file_path.exists(){
            let configuration = ConfigurationData {
                folders:vec![], 
                frequency:Frequency{frequency:String::from("")},
                username:String::from("guest"),
                private_key:String::from(""),
                pub_key:String::from("")
            };

            let configuration_json = serde_json::to_string(&configuration);
            if(!configuration_json.is_ok()){
                println!("Erreur d'écriture de données dans le fichier de conf. Veuillez redémarrer l'application");
            }
        
            let writed = write_data(&conf_path, &configuration_json.unwrap());
            if !writed.is_ok(){
                panic!("Failed to write data to conf file");
            }
        }

        // On récupère la config actuelle
        let data = read_data(&conf_path);
        if !data.is_ok() {
            println!("Erreur d'ouverture du fichier de conf. Veuillez redémarrer l'application, si le problème persiste vérifier les droits sur ce fichier : {}", &conf_path);
            panic!("erorr");
        }

        let configuration: Result<ConfigurationData> = serde_json::from_str(&data.unwrap());
        if(!configuration.is_ok()){
            panic!("error");
        }

        // On fill les attributs de l'objet avec les données de la configuration.
        let loaded_configuration = configuration.unwrap();
        self.folders = loaded_configuration.folders;
        self.frequency = loaded_configuration.frequency;
        self.username = loaded_configuration.username;
        self.private_key = loaded_configuration.private_key;
        self.pub_key = loaded_configuration.pub_key;
    }

    pub fn write_data(&mut self){
        let conf_path = conf_file::get_conf_path();

        // Si le fichier de config n'existe pas on le créer
        let configuration_json = serde_json::to_string(&self);
        let writed = write_data(&conf_path, &configuration_json.unwrap());
        if !writed.is_ok(){
            panic!("Failed to write data to conf file");
        }
        integrity::update_hash();
    }

    pub fn add_folder(&mut self, folder: FolderData) {
        let new_path = Path::new(&folder.path);

        // Check if the folder path already exists or is a subpath of an existing path
        let is_duplicate_or_subpath = self.folders.iter().any(|existing_folder| {
            let existing_path = Path::new(&existing_folder.path);
            new_path.starts_with(existing_path)
        });

        // Only add the folder if it's not a duplicate or subpath
        if !is_duplicate_or_subpath {
            self.folders.push(folder);
            self.write_data(); // Call your function to handle data persistence
        } else {
            messages::display_flash_message("Le dossier ou fichier que vous essayez d'enregistrer existe déjà ou il est présent dans un des dossier sauvegardés");
        }
    }    

    pub fn remove_folder(&mut self, path: &str){        
        self.folders.retain(|f| f.path != path); // This will remove the folder by its path
    }
}

