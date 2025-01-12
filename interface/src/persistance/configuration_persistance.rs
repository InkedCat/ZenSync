use gtk::prelude::*;
use gtk::{gio,glib, FileChooserDialog, Application, Builder, Window, ResponseType };
use std::cell::Cell;
use std::rc::Rc;
use glib::clone;
use std::cell::RefCell;
use gtk::{Button, FileChooserAction };
use std::env;

use crate::configuration_data;
use configuration_data::{ConfigurationData, Frequency, FolderData};


pub struct ConfigurationPersistance {
    configuration: Option<Rc<RefCell<ConfigurationData>>>
}

impl ConfigurationPersistance {
    pub fn get(&mut self) -> Rc<RefCell<ConfigurationData>> {
        if let Some(conf) = &self.configuration {
            // Return the existing Rc (shared ownership)
            Rc::clone(conf)
        } else {
            // Create ConfigurationData and wrap it in Rc<RefCell> directly
            let configuration = ConfigurationData {
                folders: vec![],
                frequency: Frequency {
                    frequency: String::from(""),
                },
                username:String::from("guest"),
                private_key:String::from(""),
                pub_key:String::from("")
            };
            let new_config = Rc::new(RefCell::new(configuration));
            self.configuration = Some(Rc::clone(&new_config)); // Save the Rc
            new_config
        }
    }
}
