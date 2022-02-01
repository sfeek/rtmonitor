use std::fs::File;
use std::io::{BufRead, BufReader};

// Main
fn main() {
    let mut onerow: Vec<f64>;
    let mut matrix: Vec<Vec<f64>> = Vec::new();
    let mut ms: Vec<f64> = Vec::new();


    let filename = "test.txt";

    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    // Populate the matrix
    for (_index, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        onerow = csv_split(&line);
        matrix.push(onerow);
    }

    for r in &matrix {
        for &t in r{
            print!("{} ",science_pretty_format(t, 3));
        }
        ms.push(r[8]);

        println!();
    }

    let m = mean(&ms);
    println!("Mean:{} SD: {}", science_pretty_format(m, 3), science_pretty_format(sd_sample(&ms, &m), 3));
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
