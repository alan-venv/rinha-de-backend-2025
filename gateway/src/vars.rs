pub fn trigger() -> u128 {
    std::env::var("TRIGGER")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or(200)
}

pub fn workers() -> usize {
    std::env::var("WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2)
}

pub fn socket() -> String {
    std::env::var("SOCKET").expect("socket path not set")
}
