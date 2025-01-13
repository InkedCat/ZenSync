#![allow(warnings)]
use gdk::Display;
use glib::clone;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{gio, glib, Builder, FileChooserDialog, ResponseType, Window};
use gtk::{Button, FileChooserAction};
use gtk::{CssProvider, StyleContext};
use once_cell::sync::Lazy;
use persistance::window_persistance::{change_current_window, initialize_app, initialize_css};
use std::cell::Cell;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread;
use tokio::sync::mpsc;

mod persistance;
use libadwaita::prelude::*;
use libadwaita::Application;
use persistance::window_persistance;

mod ui;
use ui::logic::welcome_setup::setup_window;

mod utils;

mod configuration_data;
use configuration_data::{ConfigurationData, FolderData, Frequency};

// Define a static-like holder for the current window, localized to the GTK main thread
thread_local! {
    pub static APP_BUILDER: RefCell<Option<Builder>> = RefCell::new(None);
    pub static APP_WINDOW: RefCell<Option<Window>> = RefCell::new(None);
    pub static APP: RefCell<Option<Application>> = RefCell::new(None);
    pub static ZSYNC_TX: RefCell<Option<mpsc::Sender<String>>> = RefCell::new(None);
    pub static ZSYNC_RX_GET: RefCell<Option<mpsc::Receiver<String>>> = RefCell::new(None);
    pub static ZSYNC_RX_ADD: RefCell<Option<mpsc::Receiver<String>>> = RefCell::new(None);
    pub static ZSYNC_RX_SYNC: RefCell<Option<mpsc::Receiver<String>>> = RefCell::new(None);
    pub static CONFIGURATION: RefCell<ConfigurationData> = RefCell::new(ConfigurationData {
        folders:vec![],
        frequency:Frequency{frequency:String::from("")},
        username:String::from("guest"),
        private_key:String::from(""),
        pub_key:String::from("")
    });
}

fn initialize_main_window(app: &Application) {
    // Load the UI file using a Builder
    let builder = Builder::from_resource("/window/welcome.ui");

    // Find the main_window object in the UI file
    let main_window: Window = builder
        .object("main_window")
        .expect("Couldn't find 'main_window' in UI file.");

    setup_window(&builder);

    APP_BUILDER.with(|app_builder| {
        *app_builder.borrow_mut() = Some(builder);
    });

    // Associate the window with the application
    main_window.set_application(Some(app));

    // Present the main window
    main_window.present();

    // Store the window in the global state
    APP_WINDOW.with(|app_window| {
        *app_window.borrow_mut() = Some(main_window);
    });
}

fn main() {
    let (mut tx_gui, mut rx_back) = mpsc::channel::<String>(4096);
    let (mut tx_back_sync, mut rx_gui_sync) = mpsc::channel::<String>(4096);
    let (mut tx_back_get, mut rx_gui_get) = mpsc::channel::<String>(4096);
    let (mut tx_back_add, mut rx_gui_add) = mpsc::channel::<String>(4096);
    let tokio_runtime = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to launch the worker thread");

        runtime.block_on(async {
            match client::handle(
                &mut rx_back,
                &mut tx_back_get,
                &mut tx_back_sync,
                &mut tx_back_add,
            )
            .await
            {
                Ok(_) => println!("Client exited successfully"),
                Err(e) => eprintln!("Client exited with error: {}", e),
            }
        });
    });

    ZSYNC_TX.with(|zsync_tx| {
        *zsync_tx.borrow_mut() = Some(tx_gui);
    });

    ZSYNC_RX_ADD.with(|zsync_rx| {
        *zsync_rx.borrow_mut() = Some(rx_gui_add);
    });

    ZSYNC_RX_GET.with(|zsync_rx| {
        *zsync_rx.borrow_mut() = Some(rx_gui_get);
    });

    ZSYNC_RX_SYNC.with(|zsync_rx| {
        *zsync_rx.borrow_mut() = Some(rx_gui_sync);
    });

    if gio::resources_register_include!("resources.gresource").is_err() {
        eprintln!("Failed to register resources. Check resource paths.");
    }

    let mut app = Application::new(Some("ZSync"), Default::default());
    // Connect activate signal during application setup
    app.connect_activate(|app| {
        // Initialize CSS, load main window, etc.
        initialize_css();
        initialize_main_window(&app);
    });

    initialize_app(&app); // Store the application in the global state

    app.run();

    tokio_runtime
        .join()
        .expect("Worker thread failed to properly close");

    return;
}
