#![allow(warnings)]
use crate::persistance::window_persistance::change_current_window;
use crate::{APP, APP_BUILDER, APP_WINDOW, CONFIGURATION};
use gdk::Display;
use glib::clone;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, Builder, Entry, FileChooserDialog, ResponseType, Window};
use gtk::{Button, FileChooserAction};
use gtk::{CssProvider, StyleContext};
use once_cell::sync::Lazy;
use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};
use openssl::x509::X509;
use std::cell::Cell;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Mutex;

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
    let confirm_btn: Button = builder
        .object("confirm_btn")
        .expect("Failed to load confirm btn");
    let cancel_btn: Button = builder
        .object("cancel_btn")
        .expect("Failed to load cancel btn");

    confirm_btn.connect_clicked(move |_| {
        change_current_window("/window/home.ui");
    });

    cancel_btn.connect_clicked(move |_| {
        change_current_window("/window/username.ui");
    });
}
