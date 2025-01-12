#![allow(warnings)]
use gdk::Display;
use glib::clone;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, Builder, FileChooserDialog, ResponseType, Window};
use gtk::{Button, FileChooserAction};
use gtk::{CssProvider, StyleContext};
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Mutex;
use crate::persistance::window_persistance::{change_current_window };
use crate::CONFIGURATION;

pub fn setup_window(builder:&Builder) {

    // Lorsqu'on clique sur le bouton start alors on affiche la fenêtre d'accueil.
    let start_btn:gtk::Button = builder.object("start_btn").unwrap();
    
    start_btn.connect_clicked(|_| {
        let mut username_exists = false;
        CONFIGURATION.with(|configuration| { 
            let mut configuration = configuration.borrow_mut();
            configuration.get_data();
             
            username_exists = configuration.username != "guest" && configuration.username != "";
        });


        if(username_exists){
            change_current_window("/window/home.ui");
        } else {
            change_current_window("/window/username.ui");
        }
    });

    let quit_btn:gtk::Button = builder.object("quit_btn").unwrap();

    quit_btn.connect_clicked(|_| {
        std::process::exit(0);
    });
}
