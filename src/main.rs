//#![windows_subsystem = "windows"]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::manual_range_contains)]
use fltk::{
    app, button::*, dialog::*, frame::*, group::*, input::*, prelude::*, text::*, window::*,
};
use ssh2::Session;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc::channel;
use std::{thread, time};

// Main
fn main() {
    // Initialize thread comms
    let (tx1, rx) = channel();

    // Initialize the GUI
    let app_handle = app::App::default();
    let mut wind = Window::new(100, 100, 700, 500, "Real Time Monitor v1.0");
    let mut output = TextDisplay::new(10, 10, 680, 360, "");
    let mut count = IntInput::new(580, 440, 54, 22, "Count");

    // Text buffers for our inputs and output
    let text = TextBuffer::default();
    output.set_buffer(Some(text));

    count.set_value("60");

    let tx2 = tx1.clone();
    let tx3 = tx1.clone();

    // Start button
    let mut start_button = Button::new(180, 420, 200, 57, "Start");
    start_button.set_callback(move |_| tx1.send(1).unwrap()); // 1 = Start

    // Stop button
    let mut stop_button = Button::new(400, 420, 100, 57, "Stop");
    stop_button.set_callback(move |_| tx2.send(2).unwrap()); // 2 = Stop

    // Show the window
    wind.end();
    wind.show();

    // Spawn a new timer thread
    thread::spawn(move || {
        // Send every 10 seconds
        loop {
            let _ = tx3.send(3); // 3 = read from device
            thread::sleep(time::Duration::from_secs(10));
        }
    });

    // Spawn a new thread to handle button controls
    thread::spawn(move || {
        // Keep receiving in a loop, until tx is dropped!
        let mut running = false;
        while let Ok(n) = rx.recv() {
            match n {
                1 => running = true,
                2 => running = false,
                3 => {
                    // If running then grab data and process it
                    if running {
                        // Connect to the local SSH server
                        let tcp = TcpStream::connect("10.168.0.6:22").unwrap();
                        let mut sess = Session::new().unwrap();
                        sess.set_tcp_stream(tcp);
                        sess.handshake().unwrap();
                        sess.userauth_password("pi", "Captain6652").unwrap();

                        // Send command to truncate to last 60 records
                        let mut channel = sess.channel_session().unwrap();
                        channel
                            .exec("tail -n 60 realtime.csv > lastminute.csv")
                            .unwrap();
                        let _ = channel.wait_close();

                        // Get the file
                        let (mut remote_file, _) =
                            sess.scp_recv(Path::new("lastminute.csv")).unwrap();

                        let mut contents = Vec::new();
                        remote_file.read_to_end(&mut contents).unwrap();

                        // Close the channel and wait for the whole content to be tranferred
                        remote_file.send_eof().unwrap();
                        remote_file.wait_eof().unwrap();
                        remote_file.close().unwrap();
                        remote_file.wait_close().unwrap();
                        let s = String::from_utf8(contents).unwrap();

                        // Show it in the window
                        output.buffer().unwrap().set_text(&format!("{}", &s));
                    }
                }
                _ => break,
            }
        }
    });

    // Enter main loop
    app_handle.run().unwrap();
}

// Calculate mean
fn mean(vec: &[f64]) -> f64 {
    let sum: f64 = Iterator::sum(vec.iter());
    sum / vec.len() as f64
}

// Calculate SD of a sample
fn sd_sample(x: &[f64], mean: &f64) -> f64 {
    let mut sd: f64 = 0.0;

    for v in x.iter() {
        sd += (v - mean).powf(2.0);
    }
    (sd / (x.len() - 1) as f64).sqrt()
}

// Calculate SD of a sample
fn sd_pop(x: &[f64], mean: &f64) -> f64 {
    let mut sd: f64 = 0.0;

    for v in x.iter() {
        sd += (v - mean).powf(2.0);
    }
    (sd / x.len() as f64).sqrt()
}

// Pretty Format Scientific Numbers
fn science_pretty_format(value: f64, digits: usize) -> String {
    if value.abs() == 0.0 {
        "0".to_string();
    }
    if value.abs() >= 10000.0 || value.abs() < 0.001 {
        format!("{:.*e}", digits, value);
    }
    format!("{:.*}", digits, value)
        .trim_end_matches(|c| c == '0')
        .trim_end_matches(|c| c == '.')
        .to_string()
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
