#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::io;

use may_minihttp::{HttpService, HttpServiceFactory, Request, Response};

struct Techempower {}

impl HttpService for Techempower {
    fn call(&mut self, _req: Request, rsp: &mut Response) -> io::Result<()> {
        rsp.header("Content-Type: text/plain").body("Hello, world!");

        Ok(())
    }
}

struct HttpServer {}

impl HttpServiceFactory for HttpServer {
    type Service = Techempower;

    fn new_service(&self, _id: usize) -> Self::Service {
        Techempower {}
    }
}

fn main() {
    may::config()
        .set_workers(4)
        .set_pool_capacity(4096)
        .set_stack_size(4096);
    println!("Starting http server: 127.0.0.1:8080");
    let server = HttpServer {};
    server.start("0.0.0.0:8080").unwrap().join().unwrap();
}
