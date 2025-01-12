use crate::APP;
use crate::APP_WINDOW;
use crate::APP_BUILDER;
use gdk::Display;
use glib::clone;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{gio, glib, Builder, FileChooserDialog, ResponseType, Window};
use gtk::{Button, FileChooserAction};
use gtk::{CssProvider, StyleContext};
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Mutex;
use crate::ui::logic;
use libadwaita::Application;

pub fn change_current_window(window_path: &str) {
    // Hide the current window
    APP_WINDOW.with(|app_window| {
        if let Some(ref window) = *app_window.borrow() {
            window.hide();
        } else {
            println!("No current window to hide.");
        }
    });

    // Load the new window from resource
    APP.with(|app| {
        if let Some(ref app_clone) = *app.borrow_mut() {
            let builder = Builder::from_resource(window_path);

            APP_BUILDER.with(|build| {
                *build.borrow_mut() = Some(builder.clone());
            });
                
            if(window_path == "/window/home.ui"){
                logic::home_setup::setup_window(&builder); 
            } else if(window_path == "/window/username.ui"){
                logic::username_setup::setup_window(&builder); 
            } else if(window_path == "/window/keys.ui"){
                logic::keys_setup::setup_window(&builder); 
            }

            let new_window: gtk::Window = builder
                .object("main_window")
                .expect("Couldn't find main_window in UI file.");

            // Set application to the new window
            new_window.set_application(Some(&*app_clone));

            new_window.set_default_size(1100, 700);
            // Update the global state
            APP_WINDOW.with(|app_window| {
                *app_window.borrow_mut() = Some(new_window.clone());
            });

            // Show the new window
            new_window.present();
        } else {
            println!("Application is not initialized.");
        }
    });
}

pub fn initialize_css() {
    let css_files = [
        "style/utility.css",
    ];

    for css_path in &css_files {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource(css_path);

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}

pub fn initialize_app(application: &Application) {
    APP.with(|app| {
        *app.borrow_mut() = Some(application.clone());
    });
}
