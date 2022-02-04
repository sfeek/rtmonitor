//#![windows_subsystem = "windows"]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::manual_range_contains)]
use fltk::{app, button::*, dialog::*, frame::*, group::*, input::*, text::*, window::*, prelude::*};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::Path;
use ssh2::Session;
use std::{thread, time};
use std::sync::mpsc::channel;

// Main
fn main() {
    // Initialize thread comms
    let (tx1, rx) = channel();

    // Initialize the GUI
    let app_handle = app::App::default();
    let mut wind = Window::new(100, 100, 700, 500, "Real Time Monitor v1.0");
    let mut output = TextDisplay::new(10, 10, 680, 360, "");
    let mut count =  IntInput::new(580, 440, 54, 22, "Count");

    // Text buffers for our inputs and output
    let text = TextBuffer::default();

    count.set_value("60");

    // Set output buffer
    output.set_buffer(Some(text));

    let tx2 = tx1.clone();
    let tx3 = tx1.clone();

    // Start button
    let mut start_button = Button::new(180, 420, 200, 57, "Start");
    start_button.set_callback(move |_| tx1.send(1).unwrap());

    // Stop button
    let mut stop_button = Button::new(400, 420, 100, 57, "Stop");
    stop_button.set_callback(move |_| tx2.send(2).unwrap());

     // Show the window
     wind.end();
     wind.show();

     // Spawn a new timer thread, and move the receiving end into the thread.
    thread::spawn(move || {
        // Send every 10 seconds
        loop {
            let _= tx3.send(3);
            thread::sleep(time::Duration::from_secs(5));
        }
    });

    // Spawn a new thread to handle button controls
    thread::spawn(move || {
        // Keep receiving in a loop, until tx is dropped!
        let mut running = false;
        while let Ok(n) = rx.recv() { // Note: `recv()` always blocks
            match n {
                1=> {output.buffer().unwrap().set_text("Running True");running = true;},
                2=> {output.buffer().unwrap().set_text("Running False");running = false;},
                3=> {
                        if running == true {
                            output.buffer().unwrap().set_text("I am running!");
                        } else {
                            output.buffer().unwrap().set_text("I am NOT running!");
                        };
                    },
                _=> break,
            }
        }
    });

    // Enter main loop
    app_handle.run().unwrap();
}

// Convert CSV from the main windows to arrays of floats, also clean up stray whitespace
fn csv_split(inp: &str) -> Vec<f64> {
    let mut values: Vec<f64> = Vec::new();

    let clean_inp: String = inp
        .replace("\n", ",")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let fields = clean_inp.split(',');

    for f in fields {
        match f.parse::<f64>() {
            Ok(v) => values.push(v),
            Err(_) => continue,
        };
    }

    values
}
