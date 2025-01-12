#![allow(warnings)]
use gdk::Display;
use glib::clone;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, Builder,Entry, FileChooserDialog, ResponseType, Window};
use gtk::{Button, FileChooserAction};
use crate::{APP, APP_BUILDER, APP_WINDOW, CONFIGURATION};
use gtk::{CssProvider, StyleContext};
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Mutex;
use crate::persistance::window_persistance::{change_current_window };

pub fn setup_window(builder: &Builder){
        // Get the username input field
        let username_input: Entry = builder.object("username_input").unwrap();
        let confirm_btn: Button = builder.object("confirm_btn").unwrap();
        let cancel_btn: Button = builder.object("cancel_btn").unwrap();


        cancel_btn.connect_clicked(move |_|{
            change_current_window("/window/welcome.ui");
        });

        confirm_btn.connect_clicked(move |_|{
            let input_value = username_input.text().to_string();
            if !input_value.trim().is_empty() {
                // Log the input when it's not empty

                // Handle configuration updates
                CONFIGURATION.with(|configuration| { 
                    let mut configuration = configuration.borrow_mut();
                    configuration.get_data();
                    configuration.username = input_value;
                    configuration.write_data();
                });

                // Change window after validation
                change_current_window("/window/keys.ui");
            } else {
                println!("Input is empty, please enter a name.");
            }
        });
}
