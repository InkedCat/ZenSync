#![allow(warnings)]
use openssl::rsa::{Rsa, Padding};
use openssl::pkey::Private;
use openssl::x509::X509;
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

//pub fn setup_window(builder: &Builder){
fn generate_keys() -> Result<(String, String), Box<dyn std::error::Error>> {
    // Generate a 2048-bit RSA key pair
    let rsa = Rsa::generate(2048)?;

    // Get the private key in PEM format
    let private_key_pem = rsa.private_key_to_pem()?;

    // Get the public key in PEM format
    let public_key_pem = rsa.public_key_to_pem()?;

    // Convert the keys to strings (from bytes)
    let private_key_str = String::from_utf8_lossy(&private_key_pem).to_string();
    let public_key_str = String::from_utf8_lossy(&public_key_pem).to_string();

    Ok((public_key_str, private_key_str))
}

pub fn setup_window(builder: &Builder) {
    match generate_keys() {
        Ok((public_key, private_key)) => {
           let pub_key_label:gtk::Entry = builder.object("public_key").expect("Failed to load pub key label"); 
           pub_key_label.set_text(&public_key);
            
           let confirm_btn: Button = builder.object("confirm_btn").expect("Failed to load confirm btn");
           let cancel_btn: Button = builder.object("cancel_btn").expect("Failed to load cancel btn");

            confirm_btn.connect_clicked(move |_|{
                CONFIGURATION.with(|conf|{
                    let mut conf = conf.borrow_mut();
                    conf.pub_key = public_key.clone();
                    conf.private_key = private_key.clone();
                });
                change_current_window("/window/home.ui");
            });

            cancel_btn.connect_clicked(move |_|{
                change_current_window("/window/username.ui");
            });
        }
        Err(e) => eprintln!("Error generating keys: {}", e),
    };

}

