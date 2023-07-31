#![allow(dead_code)]

pub const RESPONSE_HELLO_WORLD: &[u8] =
    b"HTTP/1.1 200 OK\nContent-Type: text/plain\nContent-Length: 13\n\nHello, world!";
pub const RESPONSE_SERVICE_UNAVAILABLE: &[u8] = b"HTTP/1.1 503 Service Unavailable\n\n";
