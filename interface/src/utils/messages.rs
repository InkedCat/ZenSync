use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc, Duration};
use crate::utils::conf_file;
use regex::Regex;
use crate::APP_BUILDER;
use gtk::{Button, Label, FileChooserAction, Box, Orientation,Popover, ListBox, ListBoxRow, PositionType, ApplicationWindow, Overlay, GestureClick};
use gtk::prelude::*;

#[derive(Debug)]
pub struct Message {
    pub message: String,
    pub success: bool,
    pub date: String,
    pub read: bool,
}

impl Message {
    pub fn new(message: String, success: bool, date: String, read: bool) -> Self {
        Message {
            message,
            success,
            date,
            read,
        }
    }
}


fn parse_log_line(line: &str) -> Option<Message> {
    // Define the regex pattern to extract the log details
    let re = Regex::new(r"\[(?P<date>.*?)\] Log entry: (?P<message>.*)").unwrap();
    
    if let Some(captures) = re.captures(line) {
        let date_str = &captures["date"];
        let message_str = &captures["message"];
        
        // Check if the message contains 'success' or 'succès'
        let success = message_str.to_lowercase().contains("success") || message_str.to_lowercase().contains("succès");
        
        // Extract and format the date (for now we'll just return the raw date, to format later)
        let date = format_date(date_str);
        
        // Check if the message contains [read] at the end
        let read = message_str.contains("[read]");
        
        // Clean the message text by removing '[read]'
        let cleaned_message = message_str.replace("[read]", "").trim().to_string();
        
        Some(Message::new(cleaned_message, success, date, read))
    } else {
        None
    }
}

fn format_date(date_str: &str) -> String {
    // Parse the timestamp to a DateTime object (we assume the date is in ISO 8601 format)
    let parsed_time = DateTime::parse_from_rfc3339(date_str).unwrap_or(Utc::now().into());

    // Convert it to a human-readable format (e.g., 10 seconds ago)
    let now = Utc::now();
    let duration = now.signed_duration_since(parsed_time);

    let mut result = String::new();

    if duration.num_seconds() < 60 {
        result = format!("{} seconds ago", duration.num_seconds());
    } else if duration.num_minutes() < 60 {
        result = format!("{} minutes ago", duration.num_minutes());
    } else if duration.num_hours() < 24 {
        result = format!("{} hours ago", duration.num_hours());
    } else {
        result = format!("{} days ago", duration.num_days());
    }

    result
}

pub fn read_log_file() -> io::Result<Vec<Message>> {
    let file_path = conf_file::get_log_path();
    if !Path::new(&file_path).exists() {
        // Return an empty vector if the file does not exist
        return Ok(Vec::new());
    }

    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);
    
    let mut messages = Vec::new();
    
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(message) = parse_log_line(&line) {
                messages.push(message);
            }
        }
    }
    
    Ok(messages)
}

pub fn update_logs(messages: &mut Vec<Message>) -> io::Result<()> {
    // Ensure we don't have more than 10 messages
    let file_path = conf_file::get_log_path();
    if messages.len() > 10 {
        messages.truncate(10); // Keep only the 10 most recent messages
    }

    // Add '[read]' to messages that don't have it
    for message in messages.iter_mut() {
        if !message.read {
            message.message.push_str(" [read]");
            message.read = true;
        }
    }

    // Write the updated log back to the file
    let log_content = messages.iter().map(|m| format!("[{}] Log entry: {}{}", m.date, m.message, if m.read { " [read]" } else { "" }))
        .collect::<Vec<String>>().join("\n");

    fs::write(file_path, log_content)?;

    Ok(())
}

pub fn display_flash_message(message: &str) {
    APP_BUILDER.with(move |builder|{
        let builder = builder.borrow_mut();
        let builder = builder.as_ref().unwrap();

        let main: gtk::Box = builder
            .object("banner_container")
            .expect("Failed to load main window.");

        // Load the banner UI from the resource
        let builder = gtk::Builder::from_resource("/generics/banner.ui");
        let banner: gtk::Box = builder
        .object::<gtk::Box>("banner")
        .expect("Failed to load banner widget");

        let label: gtk::Label = builder
            .object("message")
            .expect("Failed to load banner label");

        // Set the message on the label
        label.set_text(message);
        banner.show();
        label.show();

        // Create a GestureClick controller
        let click_gesture = gtk::GestureClick::new();

        // Clone the banner and add a click handler to remove it
        let banner_clone = banner.clone();
        let main_clone = main.clone();
        click_gesture.connect_pressed(move |_gesture, _n_press, _x, _y| {
            main_clone.remove(&banner_clone);
        });

        // Add the click gesture to the banner
        banner.add_controller(click_gesture);

        // Insert the banner at the beginning of the main box
        main.append(&banner);
    });
}

