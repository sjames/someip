use someip::*;

use bytes::Bytes;
use std::io::{self};

mod trait_tryout;

use trait_tryout::start_server;
use simplelog::*;

fn main() {
    println!("Hello, world!");
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )])
    .unwrap();
    start_server();
    

}
