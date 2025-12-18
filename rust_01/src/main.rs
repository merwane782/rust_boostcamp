use std::collections::HashMap;
use std::io::{self, Read};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut top_n = 10;
    let mut min_length = 1;
    let mut ignore_case = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--top" => {
                if i + 1 < args.len() {
                    top_n = args[i + 1].parse().unwrap_or(10);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--min-length" => {
                if i + 1 < args.len() {
                    min_length = args[i + 1].parse().unwrap_or(1);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--ignore-case" => {
                ignore_case = true;
                i += 1;
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            _ => i += 1,
        }
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)
        .expect("Erreur lors de la lecture de stdin");

    if input.trim().is_empty() {
        eprintln!("Erreur: aucun texte fourni");
        return;
    }

    let words: Vec<&str> = input
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .collect();

    let mut frequency: HashMap<String, usize> = HashMap::new();

    for word in words {
        let processed_word = if ignore_case {
            word.to_lowercase()
        } else {
            word.to_string()
        };

        let clean_word: String = processed_word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
            .collect();

        if clean_word.len() >= min_length {
            *frequency.entry(clean_word).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<_> = frequency.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    println!("Word frequency:");
    for (word, count) in sorted.iter().take(top_n) {
        println!("{}: {}", word, count);
    }
}

fn print_help() {
    println!("Usage: wordfreq [OPTIONS]");
    println!();
    println!("Count word frequency in text");
    println!();
    println!("Arguments:");
    println!("Text to analyze (or use stdin)");
    println!();
    println!("Options:");
    println!("--top            Show top N words [default: 10]");
    println!("--min-length     Ignore words shorter than N [default: 1]");
    println!("--ignore-case    Case insensitive counting");
    println!("-h, --help       Print help");
}
