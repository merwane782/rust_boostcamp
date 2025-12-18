use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let mut file_path = String::new();
    let mut mode = String::new();
    let mut hex_data = String::new();
    let mut offset = 0usize;
    let mut size = 0usize;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--file" => {
                if i + 1 < args.len() {
                    file_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Erreur: --file nécessite une valeur");
                    process::exit(1);
                }
            }
            "-r" | "--read" => {
                mode = "read".to_string();
                i += 1;
            }
            "-w" | "--write" => {
                if i + 1 < args.len() {
                    hex_data = args[i + 1].clone();
                    mode = "write".to_string();
                    i += 2;
                } else {
                    eprintln!("Erreur: --write nécessite une valeur");
                    process::exit(1);
                }
            }
            "-o" | "--offset" => {
                if i + 1 < args.len() {
                    let offset_str = &args[i + 1];
                    offset = if offset_str.starts_with("0x") || offset_str.starts_with("0X") {
                        match usize::from_str_radix(&offset_str[2..], 16) {
                            Ok(v) => v,
                            Err(_) => {
                                eprintln!("Erreur: offset hexadécimal invalide");
                                process::exit(1);
                            }
                        }
                    } else {
                        match offset_str.parse() {
                            Ok(v) => v,
                            Err(_) => {
                                eprintln!("Erreur: offset invalide");
                                process::exit(1);
                            }
                        }
                    };
                    i += 2;
                } else {
                    eprintln!("Erreur: --offset nécessite une valeur");
                    process::exit(1);
                }
            }
            "-s" | "--size" => {
                if i + 1 < args.len() {
                    size = match args[i + 1].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Erreur: size invalide");
                            process::exit(1);
                        }
                    };
                    i += 2;
                } else {
                    eprintln!("Erreur: --size nécessite une valeur");
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("Option inconnue: {}", args[i]);
                process::exit(1);
            }
        }
    }

    if file_path.is_empty() {
        eprintln!("Erreur: --file est obligatoire");
        process::exit(1);
    }

    if mode == "read" {
        if size == 0 {
            eprintln!("Erreur: --size est obligatoire en mode lecture");
            process::exit(1);
        }
        read_file(&file_path, offset, size);
    } else if mode == "write" {
        write_file(&file_path, offset, &hex_data);
    } else {
        eprintln!("Erreur: spécifiez --read ou --write");
        process::exit(1);
    }
}

fn read_file(path: &str, offset: usize, size: usize) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Erreur: impossible d'ouvrir le fichier");
            process::exit(1);
        }
    };

    if let Err(_) = file.seek(SeekFrom::Start(offset as u64)) {
        eprintln!("Erreur: offset invalide");
        process::exit(1);
    }

    let mut buffer = vec![0u8; size];
    match file.read_exact(&mut buffer) {
        Ok(_) => {
            let mut current_offset = offset;
            for chunk in buffer.chunks(16) {
                print!("{:08x}: ", current_offset);
                for byte in chunk {
                    print!("{:02x} ", byte);
                }
                print!(" |");
                for byte in chunk {
                    if *byte >= 32 && *byte < 127 {
                        print!("{}", *byte as char);
                    } else {
                        print!(".");
                    }
                }
                println!("|");
                current_offset += chunk.len();
            }
        }
        Err(_) => {
            eprintln!("Erreur: impossible de lire les données");
            process::exit(1);
        }
    }
}

fn write_file(path: &str, offset: usize, hex_str: &str) {
    let bytes = match hex_to_bytes(hex_str) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Erreur: chaîne hexadécimale invalide");
            process::exit(1);
        }
    };

    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Erreur: impossible d'ouvrir le fichier");
            process::exit(1);
        }
    };

    if let Err(_) = file.seek(SeekFrom::Start(offset as u64)) {
        eprintln!("Erreur: offset invalide");
        process::exit(1);
    }

    match file.write_all(&bytes) {
        Ok(_) => {
            println!("Writing {} bytes at offset 0x{:08x}", bytes.len(), offset);
            println!("Hex: {}", hex_to_display(&bytes));
            println!("ASCII: {}", bytes_to_ascii(&bytes));
            println!("✓ Successfully written");
        }
        Err(_) => {
            eprintln!("Erreur: impossible d'écrire les données");
            process::exit(1);
        }
    }
}

fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>, ()> {
    let hex_str = hex_str.trim();
    if hex_str.len() % 2 != 0 {
        return Err(());
    }

    let mut bytes = Vec::new();
    for i in (0..hex_str.len()).step_by(2) {
        let byte_str = &hex_str[i..i + 2];
        match u8::from_str_radix(byte_str, 16) {
            Ok(b) => bytes.push(b),
            Err(_) => return Err(()),
        }
    }
    Ok(bytes)
}

fn hex_to_display(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| {
            if *b >= 32 && *b < 127 {
                *b as char
            } else {
                '.'
            }
        })
        .collect()
}

fn print_help() {
    println!("Usage: hextool [OPTIONS]");
    println!();
    println!("Read and write binary files in hexadecimal");
    println!();
    println!("Options:");
    println!("-f, --file         Target file");
    println!("-r, --read         Read mode (display hex)");
    println!("-w, --write        Write mode (hex string to write)");
    println!("-o, --offset       Offset in bytes (decimal or 0x hex)");
    println!("-s, --size         Number of bytes to read");
    println!("-h, --help         Print help");
}
