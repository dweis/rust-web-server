use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

use rust_web_server::{handlers::*, router::Router, thread_pool::ThreadPool, types::*};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(8);

    let router = Router::new()
        .add_route("/", RequestHandler::StaticFile("index.html".into()))
        .fallback(RequestHandler::StaticFile("404.html".into()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let router = router.clone();

        pool.execute(|| {
            handle_connection(router, stream);
        });
    }
}

fn handle_connection(router: Router, mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let http_request = HttpRequest::try_from(
        &buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<String>>(),
    );

    let response = router.handle(http_request.unwrap());
    stream
        .write_all(response.response_text().as_bytes())
        .unwrap();
}
