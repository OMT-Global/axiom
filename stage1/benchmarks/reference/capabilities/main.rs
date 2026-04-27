use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::ToSocketAddrs;
use std::time::{SystemTime, UNIX_EPOCH};

fn pseudo_sha_shape(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn main() {
    match fs::read_to_string("stage1/examples/capabilities/src/fixture.txt") {
        Ok(value) => println!("{}", value),
        Err(_) => println!("missing"),
    }

    println!("{}", "localhost:0".to_socket_addrs().is_ok());
    println!("{}", pseudo_sha_shape("abc"));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    println!("{}", now > 0);

    match env::var("__AXIOM_STAGE1_MISSING__") {
        Ok(value) => println!("{}", value),
        Err(_) => println!("none"),
    }
}
