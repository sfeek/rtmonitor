#![windows_subsystem = "windows"]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::manual_range_contains)]
#![allow(dead_code)]

use fltk::{app, button::*, enums::*, frame::*, input::*, prelude::*, text::*, window::*};
use ssh2::Session;
use std::cmp::Ordering;
use std::f64;
use std::io::prelude::*;
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
    let mut wind = Window::new(100, 100, 800, 750, "Real Time Monitor v1.0");
    let mut output = TextDisplay::new(10, 10, 780, 610, "");
    let mut ip = Input::new(620, 640, 84, 22, "IP address");
    let mut zs = FloatInput::new(620, 670, 54, 22, "ZScore Thresh");
    let pass = SecretInput::new(620, 700, 100, 22, "Password");
    let mut status = Frame::new(10, 690, 150, 17, "Status: Stopped");

    // Text buffers for our inputs and output
    let text = TextBuffer::default();
    output.set_buffer(Some(text));

    // Prefill the fields
    zs.set_value("3.0");
    ip.set_value("10.168.0.6");

    // Set Font
    output.set_text_font(Font::Screen);

    let tx2 = tx1.clone();
    let tx3 = tx1.clone();

    // Start button
    let mut start_button = Button::new(180, 670, 200, 57, "Start");
    start_button.set_callback(move |_| tx1.send(1).unwrap()); // 1 = Start

    // Stop button
    let mut stop_button = Button::new(400, 670, 100, 57, "Stop");
    stop_button.set_callback(move |_| tx2.send(2).unwrap()); // 2 = Stop

    // Show the window
    wind.end();
    wind.show();

    // Spawn a new timer thread
    thread::spawn(move || {
        // Send every 10 seconds
        loop {
            let _ = tx3.send(3); // 3 = read from device
            thread::sleep(time::Duration::from_secs(5));
        }
    });

    // Spawn a new thread to handle button controls
    thread::spawn(move || {
        let mut connected = false;
        let mut sess: ssh2::Session = Session::new().unwrap();

        // Wait for messages from the channel
        while let Ok(n) = rx.recv() {
            match n {
                1 => {
                    status.set_label("Status: Connecting");
                    
                    // Connect to the RPi SSH server
                    let tcp = match TcpStream::connect(&format!("{}:22", &ip.value())) {
                        Ok(tcp) => tcp,
                        Err(_) => {
                            status.set_label("Status: Fail Connect");
                            app::awake();
                            continue;
                        }
                    };

                    // Create new session and attach to TCP stream
                    sess = Session::new().unwrap();
                    sess.set_tcp_stream(tcp);

                    match sess.handshake() {
                        Ok(_) => {}
                        Err(_) => {
                            status.set_label("Status: Fail Handshake");
                            app::awake();
                            continue;
                        }
                    }

                    match sess.userauth_password("pi", &pass.value()) {
                        Ok(_) => {
                            status.set_label("Status: Running");
                        }
                        Err(_) => {
                            status.set_label("Status: Fail Password");
                            app::awake();
                            continue;
                        }
                    };

                    app::awake();
                    connected = true;
                }

                2 => {
                    status.set_label("Status: Stopped");
                    if connected {
                        connected = false;
                    }
                }

                3 => {
                    // If running then grab data and process it
                    if connected {
                        // Send command to truncate to last n records
                        let mut channel = sess.channel_session().unwrap();
                        channel
                            .exec("tail -n 60 realtime.csv > lastminute.csv")
                            .unwrap();
                        channel.send_eof().unwrap();
                        channel.wait_eof().unwrap();
                        channel.close().unwrap();
                        channel.wait_close().unwrap();

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
                        output
                            .buffer()
                            .unwrap()
                            .set_text(&process_data(&s, zs.value()).to_string());

                        // Run the event loop on the main thread to refresh the screen
                        app::awake();
                    }
                }
                _ => {} // Don't do anything if an invalid number is received on the channel
            }
        }
    });

    // Enter main loop
    app_handle.run().unwrap();
}

//rec#,touch,flame,metal,motion,ir,visible,uv,distance,events,temperature,humidity,barometer,dewpoint,emf,ion,plasma,rad/cps,accelX,accelY,accelZ,accelSum,gyroX,gyroY,gyroZ,gyroSum,magX,magY,magZ,magSum

// Process data
fn process_data(ins: &str, zs: String) -> String {
    let mut output: String = String::new();
    let mut matrix: Vec<Vec<f64>> = Vec::new();
    let headings: Vec<&str> = vec![
        "Rec #",
        "Touch",
        "Flame",
        "Metal",
        "Motion",
        "Events",
        "IR",
        "Visible",
        "UV",
        "Distance",
        "Temperature",
        "Humidity",
        "Barometer",
        "Dew Point",
        "EMF",
        "Ion",
        "Plasma",
        "Radiation",
        "Acceleration",
        "Gyro",
        "GMF",
    ];
    let columns: Vec<usize> = vec![
        0, 1, 2, 3, 4, 9, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 21, 25, 29,
    ];
    let fields: Vec<&str> = vec!["Mean", "Median", "SD", "Min", "Max", "Z Count"];

    // Populate the matrix
    let lines: Vec<String> = newline_split(ins);
    for s in lines {
        let onerow = csv_split(&s);
        matrix.push(onerow);
    }

    // Process first four binary outputs
    for i in 1..5 {
        let mut c = false;
        for v in &matrix {
            if v[columns[i]] > 0.0 {
                c = true;
            }
        }
        output.push_str(&format!("{}{}\n\n", &add_spaces(headings[i], 14), c));
    }

    // Process Events because it is a non-standard field
    let status: String;

    let mut ms: Vec<f64> = Vec::new();
    for v in &matrix {
        ms.push(v[9]);
    }
    let (_, max) = min_max(&ms);
    match max as i32 {
        0 => status = "None".to_string(),
        1 => status = "Session".to_string(),
        2 => status = "Button".to_string(),
        3 => status = "Session + Button".to_string(),
        _ => status = "Error".to_string(),
    }

    output.push_str(&format!("{}{}\n\n\n\n", &add_spaces("Events", 14), status));

    // Process floating point fields 6 to 20

    // Output floating point field headers
    output.push_str(&add_spaces("Parameter", 14));
    for f in fields {
        output.push_str(&add_spaces(f, 10));
    }
    output.push_str("\n\n");

    // Output all the floating point data fields
    for i in 6..21 {
        let mut ms: Vec<f64> = Vec::new();

        for v in &matrix {
            ms.push(v[columns[i]]);
        }

        let m = mean(&ms);
        let (min, max) = min_max(&ms);

        output.push_str(&format!(
            "{}{}{}{}{}{}{}\n\n",
            &add_spaces(headings[i], 14),
            &add_spaces(&science_pretty_format(&m, 3), 10),
            &add_spaces(&science_pretty_format(&median(&ms), 3), 10),
            &add_spaces(&science_pretty_format(&sd_pop(&ms, &m), 3), 10),
            &add_spaces(&science_pretty_format(&min, 3), 10),
            &add_spaces(&science_pretty_format(&max, 3), 10),
            cnt_zscore(&zscore(&ms), &zs.parse::<f64>().unwrap())
        ));
    }

    output
}

// Add Spaces to pad strings
fn add_spaces(inp: &str, w: usize) -> String {
    let s = inp.to_string();
    let l = s.len();
    if l >= w {
        return s;
    }

    let mut output: String = String::new();

    output.push_str(&s);

    for _ in 0..(w - l) {
        output.push(' ');
    }

    output
}

// Calculate median
fn median(vec: &[f64]) -> f64 {
    let mut v = vec.to_owned();

    v.sort_by(cmp_f64);
    v[vec.len() / 2]
}

// Calculate min, max
fn min_max(vec: &[f64]) -> (f64, f64) {
    let mut v = vec.to_owned();

    if v.is_empty() {
        return (0.0, 0.0);
    }
    v.sort_by(cmp_f64);

    (v[0], v[v.len() - 1])
}

// Calculate ZScore
fn zscore(vec: &[f64]) -> Vec<f64> {
    let v = vec.to_owned();
    let mut output: Vec<f64> = Vec::new();

    let avg = mean(&v);
    let sd = sd_pop(&v, &avg);

    for val in v {
        output.push((val - avg) / sd);
    }

    output
}

// Count number of Z Scores > threshhold
fn cnt_zscore(zs: &[f64], t: &f64) -> i32 {
    let mut c = 0;
    for z in zs {
        if z.abs() >= *t {
            c += 1;
        };
    }
    c
}

// Comparison function for vec<64> sorting
fn cmp_f64(a: &f64, b: &f64) -> Ordering {
    if a.is_nan() {
        return Ordering::Greater;
    }
    if b.is_nan() {
        return Ordering::Less;
    }
    if a < b {
        return Ordering::Less;
    } else if a > b {
        return Ordering::Greater;
    }
    Ordering::Equal
}

// Calculate mean
fn mean(vec: &[f64]) -> f64 {
    let sum: f64 = Iterator::sum(vec.iter());
    sum / vec.len() as f64
}

// Calculate SD of a population
fn sd_pop(x: &[f64], mean: &f64) -> f64 {
    let mut sd: f64 = 0.0;

    for v in x.iter() {
        sd += (v - mean).powf(2.0);
    }
    (sd / x.len() as f64).sqrt()
}

// Pretty Format Scientific Numbers
fn science_pretty_format(value: &f64, digits: usize) -> String {
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

// Convert one large string into lines 
fn newline_split(inp: &str) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();

    let r = inp.split('\n');

    for row in r {
        if !row.is_empty() {
            rows.push(row.to_string());
        }
    }

    rows
}
