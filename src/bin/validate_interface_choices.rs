use std::{env, process};

#[allow(dead_code)]
#[path = "../options/interface_choices.rs"]
mod interface_choices;

use interface_choices::InterfaceChoices;

fn main() {
    let mut args = env::args().skip(1);

    let Some(path) = args.next() else {
        eprintln!("usage: cargo run --bin validate_interface_choices -- <path-to-json>");
        process::exit(2);
    };

    if args.next().is_some() {
        eprintln!("error: expected exactly one argument");
        process::exit(2);
    }

    match std::fs::read_to_string(&path).and_then(|json| {
        serde_json::from_str::<InterfaceChoices>(&json).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
        })
    }) {
        Ok(_) => {
            println!("{path} is valid InterfaceChoices JSON");
        }
        Err(error) => {
            eprintln!("validation failed for {path}: {error}");
            process::exit(1);
        }
    }
}