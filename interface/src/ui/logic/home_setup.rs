#![allow(warnings)]

use crate::configuration_data::{ConfigurationData, FileType, FolderData, Frequency};
use crate::persistance::window_persistance::change_current_window;
use crate::utils::display::format_file_size;
use crate::utils::{cron, display, integrity, messages};
use crate::{APP, APP_BUILDER, APP_WINDOW, CONFIGURATION, ZSYNC_TX};
use gdk::Display;
use glib::clone;
use glib::{timeout_add_local, timeout_add_seconds, ControlFlow::Continue};
use gtk::gdk;
use gtk::glib::property::PropertySet;
use gtk::glib::MainContext;
use gtk::prelude::*;
use gtk::{gio, glib, Builder, FileChooserDialog, ResponseType, StringList, Window};
use gtk::{
    ApplicationWindow, Box, Button, FileChooserAction, GestureClick, Label, ListBox, ListBoxRow,
    Orientation, Overlay, Popover, PositionType,
};
use gtk::{CssProvider, StyleContext};
use messages::{read_log_file, update_logs, Message};
use once_cell::sync::Lazy;
use reqwest::Error;
use std::cell::Cell;
use std::cell::RefCell;
use std::process::{exit, Command};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{env, thread};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

pub fn setup_window(builder: &Builder) {
    // Vérifie l'intégrité de l'application.
    let is_conf_file_valid = integrity::check_integrity();
    if (!is_conf_file_valid) {
        messages::display_flash_message(
            "Attention : le fichier de configuration a été modifié en dehors de l'application.",
        );
    }
    integrity::check_integrity();
    integrity::update_hash();

    // Setup l'état de connexion de l'application (offline ou online)
    setup_ping(builder);

    // Setup boites de receptions
    let messages = messages::read_log_file();
    let messages = messages.unwrap();
    setup_messages_box_popover(&messages);

    // Setup settings
    let builder_clone = builder.clone();
    let settings_btn: gtk::Button = builder.object("settings_btn").expect("Failed to load btn");
    settings_btn.connect_clicked(move |_| {
        display_setting_window(&builder_clone);
    });

    // Setup les actions
    let container: gtk::FlowBox = builder.object("folders_container").unwrap();
    let select_button: Button = builder.object("select_folder_button").unwrap();
    let select_file_button: Button = builder.object("select_file_button").unwrap();
    let synchronize_button: Button = builder.object("synchronize_button").unwrap();
    let restore_button: Button = builder.object("restore_button").unwrap();
    let welcome_sentence: Label = builder.object("welcome_sentence").unwrap();

    CONFIGURATION.with(|configuration| {
        let mut configuration = configuration.borrow_mut();
        configuration.get_data();
        welcome_sentence.set_text(&format!(
            "Welcome back, {}.",
            configuration.username.clone()
        ));
        update_folders_displayed(&configuration);
    });

    // Quand bouton de séléction est cliqué on affiche la boite de dialogue séléction de folders.
    select_button.connect_clicked(clone!(move |_| {
        // Create the folder chooser dialog
        let dialog = FileChooserDialog::builder()
            .title("Select a Folder")
            .action(FileChooserAction::SelectFolder)
            .modal(true)
            .build();

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Open", ResponseType::Accept);

        // Handle response from the dialog
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                // Si un user a sélectionner un dossier alors on le traite.
                if let Some(folder) = dialog.file().and_then(|f| f.path()) {
                    // On récupère le chemin du fichier sélectionné
                    let folder_path = folder.into_os_string().into_string().unwrap();

                    // On créer un nouveau élément de sauvegarde dossier et on l'enregistre
                    CONFIGURATION.with(|configuration| {
                        let folder = FolderData::new(folder_path, false, FileType::Folder)
                            .expect("Erreur d'ajout du fichier.");
                        let mut configuration = configuration.borrow_mut();

                        // On enregistres le nouveau fichier
                        configuration.add_folder(folder);
                        update_folders_displayed(&configuration);
                        configuration.write_data();
                    });
                }
            }
            dialog.close();
        });

        dialog.show();
    }));

    select_file_button.connect_clicked(clone!(move |_| {
        // Create the folder chooser dialog
        let dialog = FileChooserDialog::builder()
            .title("Select a Folder")
            .modal(true)
            .build();

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Open", ResponseType::Accept);

        // Handle response from the dialog
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                // Si un user a sélectionner un dossier alors on le traite.
                if let Some(folder) = dialog.file().and_then(|f| f.path()) {
                    // On récupère le chemin du fichier sélectionné
                    let folder_path = folder.into_os_string().into_string().unwrap();

                    // On créer un nouveau élément de sauvegarde dossier et on l'enregistre
                    CONFIGURATION.with(|configuration| {
                        let folder = FolderData::new(folder_path, false, FileType::File)
                            .expect("Erreur d'ajout du fichier.");
                        let mut configuration = configuration.borrow_mut();

                        // On enregistres le nouveau fichier
                        configuration.add_folder(folder);
                        update_folders_displayed(&configuration);
                        configuration.write_data();
                    });
                }
            }
            dialog.close();
        });

        dialog.show();
    }));

    synchronize_button.connect_clicked(clone!(move |_| {
        CONFIGURATION.with(|configuration| {
            let mut configuration = configuration.borrow_mut();
            let folders = configuration.folders.clone();

            let mut folder_list: Vec<String> = Vec::new();
            let mut file_list: Vec<String> = Vec::new();
            for folder in folders {
                match folder.file_type {
                    FileType::Folder => folder_list.push(folder.path),
                    FileType::File => file_list.push(folder.path),
                }
            }

            let mut files = None;
            if file_list.len() > 1 {
                files = Some(file_list.join(" "));
            } else if let Some(first) = file_list.get(0) {
                files = Some(first.clone());
            }

            let mut folders = None;
            if folder_list.len() > 1 {
                folders = Some(folder_list.join(" "));
            } else if let Some(first) = folder_list.get(0) {
                folders = Some(first.clone());
            }

            ZSYNC_TX.with(|tx| {
                let tx = tx.borrow_mut().clone();
                thread::spawn(move || {
                    if let Some(tx) = tx {
                        if let Some(files) = files {
                            let full_command = format!("get_all {}", files);
                            match tx.blocking_send(full_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                        if let Some(folders) = folders {
                            let full_command = format!("get_all {}", folders);
                            match tx.blocking_send(full_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }

                        for file in file_list {
                            let add_command = format!("add {}", file);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }

                        for folder in folder_list {
                            let add_command = format!("add_folder {}", folder);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                    }
                })
            })
        });
    }));

    restore_button.connect_clicked(clone!(move |_| {
        CONFIGURATION.with(|configuration| {
            let mut configuration = configuration.borrow_mut();
            let folders = configuration.folders.clone();

            let mut folder_list: Vec<String> = Vec::new();
            let mut file_list: Vec<String> = Vec::new();
            for folder in folders {
                match folder.file_type {
                    FileType::Folder => folder_list.push(folder.path),
                    FileType::File => file_list.push(folder.path),
                }
            }

            ZSYNC_TX.with(|tx| {
                let tx = tx.borrow_mut().clone();
                thread::spawn(move || {
                    if let Some(tx) = tx {
                        for file in file_list {
                            let add_command = format!("sync {}", file);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }

                        for folder in folder_list {
                            let add_command = format!("sync_folder {}", folder);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                    }
                })
            })
        });
    }));
}

pub fn create_folder_element(folder_data: FolderData) -> gtk::Box {
    // Load the folder UI from the XML file
    let builder = Builder::from_resource("/component/folder.ui");

    // Get folder UI components
    let folder: gtk::Box = builder.object("folder").expect("Failed to load folder box");
    let folder_name: gtk::Label = builder
        .object("folder_name")
        .expect("Failed to load folder_name");
    let folder_size: gtk::Label = builder
        .object("folder_size")
        .expect("Failed to load folder_size");
    let folder_icon: gtk::Image = builder.object("icon").expect("Failed to load folder icon");

    // Set icon type
    match folder_data.file_type {
        FileType::Folder => folder_icon.set_icon_name(Some("folder")),
        FileType::File => folder_icon.set_icon_name(Some("text-x-generic")),
    }

    // Set folder info
    let size_display = format_file_size(folder_data.size);
    folder_name.set_label(&folder_data.get_name());
    folder_size.set_label(&format!("Size: {}", size_display));

    // Dropdown
    let dropdown_button: gtk::Button = builder
        .object("dropdown_btn")
        .expect("Failed to load dropdown_btn");
    // Create a popover
    let popover = Popover::builder().build();

    // Example content for the popover
    let button_box = Box::new(Orientation::Vertical, 5);

    // Create individual buttons and add them to the box
    let delete_button = Button::builder().label("Delete").build();
    let sync_button = Button::builder().label("Sync").build();
    let restore_button = Button::builder().label("Restore").build();
    let properties_button = Button::builder().label("Properties").build();

    let folder_data_clone = folder_data.clone();
    sync_button.connect_clicked(move |btn| {
        ZSYNC_TX.with(|tx| {
            let tx = tx.borrow_mut().clone();
            let folder_data_clone = folder_data_clone.clone();

            thread::spawn(move || {
                if let Some(tx) = tx {
                    match folder_data_clone.file_type {
                        FileType::Folder => {
                            let sync_command = format!("sync_folder {}", folder_data_clone.path);
                            match tx.blocking_send(sync_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                        FileType::File => {
                            let sync_command = format!("sync {}", folder_data_clone.path);
                            match tx.blocking_send(sync_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                    }
                }
            })
        });
    });

    let folder_data_clone = folder_data.clone();
    restore_button.connect_clicked(move |btn| {
        ZSYNC_TX.with(|tx| {
            let tx = tx.borrow_mut().clone();
            let folder_data_clone = folder_data_clone.clone();

            thread::spawn(move || {
                if let Some(tx) = tx {
                    match folder_data_clone.file_type {
                        FileType::Folder => {
                            let add_command = format!("add_folder {}", folder_data_clone.path);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                        FileType::File => {
                            let add_command = format!("add {}", folder_data_clone.path);
                            match tx.blocking_send(add_command) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending command to zsync: {}", e);
                                }
                            }
                        }
                    }
                }
            })
        });
    });

    let folder_data_clone = folder_data.clone();
    delete_button.connect_clicked(move |btn| {
        CONFIGURATION.with(|conf| {
            let mut conf = conf.borrow_mut();
            conf.remove_folder(&folder_data_clone.path);
            conf.write_data();
            update_folders_displayed(&conf);
        });

        ZSYNC_TX.with(|tx| {
            let tx = tx.borrow_mut().clone();
            let folder_data_clone = folder_data_clone.clone();

            thread::spawn(move || {
                if let Some(tx) = tx {
                    let command = format!("remove {}", &folder_data_clone.path);
                    match tx.blocking_send(command) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error sending command to zsync: {}", e);
                        }
                    }
                }
            })
        });
    });

    properties_button.connect_clicked(move |btn| {
        display_properties_window(&folder_data);
    });

    button_box.append(&properties_button);
    button_box.append(&delete_button);
    button_box.append(&sync_button);
    button_box.append(&restore_button);

    popover.set_child(Some(&button_box));
    popover.set_has_arrow(false);

    // On affiche le popover de façon relative à la position du boutton
    dropdown_button.connect_clicked(move |btn| {
        popover.set_parent(btn); // Attach popover to the button
        popover.popup();
    });

    folder
}

pub fn update_folders_displayed(configuration: &ConfigurationData) {
    APP_BUILDER.with(|builder| {
        let builder = builder.borrow();
        let builder = builder.as_ref().unwrap();

        let container: gtk::FlowBox = builder
            .object("folders_container")
            .expect("Failed to load folder box");
        container.remove_all();

        let folders = configuration.folders.clone();

        for folder in folders {
            let folder_element = create_folder_element(folder);
            folder_element.set_size_request(275, -1); // Fixed width, flexible height
            container.insert(&folder_element, -1)
        }

        // Section stats
        let mut elements_sync_count = 0;
        let synchronization_rate: gtk::Label = builder
            .object("synchronization_rate")
            .expect("Failed to load folder box");

        let elements_count: gtk::Label = builder
            .object("folders_count")
            .expect("Failed to load folders count");

        let folders = configuration.folders.clone();
        for folder in &folders {
            if folder.is_sync {
                elements_sync_count += 1;
            }
        }

        // S'il n'y a rien à afficher on affiche (no folders...)
        // Sinon on affiche les éléments.
        let no_folders: gtk::Box = builder
            .object("no_folders")
            .expect("Failed to load no folders");
        container.hide();

        if (folders.len() == 0) {
            no_folders.show();
            container.hide();
        } else {
            no_folders.hide();
            container.show();
        }

        elements_count.set_text(&format!("{}", folders.len()));

        let rate = if folders.len() > 0 {
            (elements_sync_count as f64) / (folders.len() as f64) * 100.0
        } else {
            0.0
        };

        synchronization_rate.set_text(&format!("{:.1}%", rate));
    });
}

// Create a function to generate a message widget
fn create_message_widget(message: &Message) -> gtk::Box {
    // Create the main horizontal box
    let main_box = gtk::Box::new(Orientation::Horizontal, 16);

    // Create the image based on the type of message
    let icon_name = if message.success {
        "adw-entry-apply-symbolic" // Success icon
    } else {
        "dialog-error-symbolic" // Failure icon
    };
    let image = gtk::Image::from_icon_name(&icon_name);
    image.set_margin_start(8);
    image.set_opacity(0.5);
    image.set_pixel_size(20);

    // Add the image to the main box
    main_box.append(&image);

    // Create the vertical box for the text
    let text_box = gtk::Box::new(Orientation::Vertical, 4);
    text_box.set_margin_end(6);

    // Create and add the message label
    let message_label = Label::new(Some(&message.message));
    text_box.append(&message_label);

    // Create and add the date label
    let date_label = Label::new(Some(&message.date));
    date_label.set_halign(gtk::Align::Start);
    date_label.set_opacity(0.5);
    text_box.append(&date_label);

    // Add the text box to the main box
    main_box.append(&text_box);

    main_box
}

// Function to populate the popover with messages
fn setup_messages_box_popover(messages: &[Message]) {
    let messages_box_btn = APP_BUILDER.with(|builder| {
        let builder = builder.borrow_mut();
        let builder = builder.as_ref().unwrap();
        builder
            .object::<gtk::Button>("messages_box_btn")
            .expect("Failed to load component")
            .clone()
    });

    let popover = Popover::builder().build();

    popover.set_has_arrow(false);

    // Create a vertical box to hold the message widgets
    let container = gtk::Box::new(Orientation::Vertical, 10);

    // Generate widgets for each message
    for message in messages {
        let message_widget = create_message_widget(message);
        container.append(&message_widget);
    }

    // Add the container to the popover
    popover.set_child(Some(&container));

    messages_box_btn.connect_clicked(move |btn| {
        let btn_allocation = btn.allocation(); // Get button's size and position
        let x_offset = 0; // Align to the left edge of the button
        let y_offset = btn_allocation.height() as i32 + 30; // Position below the button with a small gap

        popover.set_margin_start(x_offset); // No horizontal offset
        popover.set_margin_top(100); // Position below the button
        popover.set_parent(btn); // Attach popover to the button
        popover.popup();
    });
}

pub fn display_setting_window(builder: &Builder) {
    let widget_builder = Builder::from_resource("/window/preferences-page.ui");
    let widget: gtk::Widget = widget_builder
        .object("widget")
        .expect("Failed to load preferences widget");
    let dropdown: gtk::DropDown = widget_builder
        .object("dropdown")
        .expect("Failed to load builder.");
    let username: gtk::Label = widget_builder
        .object("username")
        .expect("Failed to load usernmae.");

    widget.set_width_request(400);
    widget.set_height_request(550);
    widget.show();

    let mut selected_frequency: u32 = 0;

    CONFIGURATION.with(|configuration| {
        let mut configuration = configuration.borrow_mut();
        configuration.get_data();

        username.set_text(&configuration.username);
        if let Some(index) = &Frequency::valid_frequencies
            .iter()
            .position(|&x| x == configuration.frequency.frequency)
        {
            selected_frequency = *index as u32;
        }
    });

    let options = StringList::new(&Frequency::valid_frequencies);
    dropdown.set_model(Some(&options));
    dropdown.set_selected(selected_frequency);

    // Connect to handle selection changes
    dropdown.connect_selected_notify(move |dropdown| {
        if let selected_index = dropdown.selected() {
            if let Some(selected_item) = options.string(selected_index as u32) {
                CONFIGURATION.with(|configuration| {
                    let mut configuration = configuration.borrow_mut();
                    cron::create_cron_file(&selected_item.to_string());
                    configuration.frequency.frequency = selected_item.to_string();
                    configuration.write_data();
                });
            }
        }
    });
}

fn display_properties_window(folder_data: &FolderData) {
    let builder = Builder::from_resource("/component/properties.ui");
    let widget: gtk::Widget = builder
        .object("widget")
        .expect("Failed to load properties widget");
    widget.set_width_request(400);
    widget.set_height_request(500);

    let folder_name: gtk::Label = builder
        .object("folder_name")
        .expect("Failed to load files count label");
    let folder_icon: gtk::Image = builder
        .object("folder_icon")
        .expect("Failed to load files count label");
    let files_count: gtk::Label = builder
        .object("files_count")
        .expect("Failed to load files count label");
    let folders_count: gtk::Label = builder
        .object("folders_count")
        .expect("Failed to load files count label");
    let folder_size: gtk::Label = builder
        .object("folder_size")
        .expect("Failed to load files count label");

    let folder_info = folder_data.get_folder_info().unwrap();
    files_count.set_text(&format!("{:?}", folder_info.files_count));
    folders_count.set_text(&format!("{:?}", folder_info.folders_count));
    folder_size.set_text(&format!("{}", format_file_size(folder_info.size)));
    folder_name.set_text(&folder_data.get_name());

    match folder_data.file_type {
        FileType::Folder => folder_icon.set_icon_name(Some("folder")),
        FileType::File => folder_icon.set_icon_name(Some("text-x-generic")),
    }
    widget.show();
}

fn setup_ping(builder: &gtk::Builder) {
    // Define the duration for pinging the server (30 seconds)
    let duration = Duration::from_secs(10);

    // Wrap the connection_label in Arc<Mutex<>> to share across threads safely
    let connection_label = Arc::new(Mutex::new(
        builder
            .object::<gtk::Label>("connection_info")
            .expect("Failed to load connection_label"),
    ));
    let connection_pulse = Arc::new(Mutex::new(
        builder
            .object::<gtk::DrawingArea>("connection_pulse")
            .expect("Failed to load connection_pulse"),
    ));
    timeout_add_local(duration, move || {
        // Perform the ping operation
        let is_online = ping_server();

        // Use MainContext to safely update the label from the main thread
        let label_clone = connection_label.clone();
        let label_pulse = connection_pulse.clone();
        glib::MainContext::default().invoke_local(move || {
            // Lock the Mutex to safely access the label
            let label = label_clone.lock().unwrap();
            let pulse = label_pulse.lock().unwrap();
            let style_context = pulse.style_context();

            // Update the label based on the result
            if is_online {
                style_context.remove_class("pulse-danger");
                style_context.add_class("pulse");
                label.set_text("online");
            } else {
                style_context.remove_class("pulse");
                style_context.add_class("pulse-danger");
                label.set_text("offline");
            }

            // Continue executing this function every 30 seconds
        });

        // Return ControlFlow to keep the timeout active
        glib::ControlFlow::Continue
    });
}

fn ping_server() -> bool {
    // Run the system's `ping` command to ping google.com
    let output = Command::new("ping")
        .arg("-c 1") // Send 1 packet
        .arg("google.com")
        .output();

    match output {
        Ok(output) => {
            // Check if the ping was successful by examining the exit status
            if output.status.success() {
                return true;
            } else {
                return false;
            }
        }
        Err(e) => {
            println!("Failed to execute ping command: {}", e);
            return false;
        }
    }
}
