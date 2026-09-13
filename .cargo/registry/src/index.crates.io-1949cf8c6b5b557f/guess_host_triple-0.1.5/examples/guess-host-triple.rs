extern crate env_logger;
extern crate guess_host_triple;

use std::process;

fn main() {
    env_logger::init();

    guess_host_triple::guess_host_triple()
        .map(|triple| println!("{}", triple))
        .unwrap_or_else(|| {
            eprintln!("Could not determine host triple");
            process::exit(1);
        });
}
