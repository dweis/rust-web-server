use std::{
    fmt::Debug,
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

use rust_web_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(8);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

type Header = (String, String);

#[derive(Debug, PartialEq)]
enum HttpMethod {
    GET,
    POST,
}

impl TryFrom<&str> for HttpMethod {
    type Error = HttpRequestParseError;
    fn try_from(s: &str) -> Result<HttpMethod, HttpRequestParseError> {
        match s {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            _ => Err(HttpRequestParseError::UnknownMethod),
        }
    }
}

#[derive(Debug)]
enum HttpVersion {
    Http10,
    Http11,
}

impl HttpVersion {
    fn to_string(self: &HttpVersion) -> &'static str {
        match *self {
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
        }
    }
}


impl TryFrom<&str> for HttpVersion {
    type Error = HttpRequestParseError;
    fn try_from(s: &str) -> Result<HttpVersion, HttpRequestParseError> {
        match s {
            "HTTP/1.0" => Ok(HttpVersion::Http10),
            "HTTP/1.1" => Ok(HttpVersion::Http11),
            _ => Err(HttpRequestParseError::UnsupportedVersion(s.into())),
        }
    }
}

// Request
// Method Request-URI HTTP-Version CRLF
// headers CRLF
// message-body
#[derive(Debug)]
struct HttpRequest {
    method: HttpMethod,
    path: String,
    version: HttpVersion,
    headers: Vec<Header>,
    body: String,
}

#[derive(Debug)]
enum HttpRequestParseError {
    UnknownMethod,
    UnsupportedVersion(String),
    IncompleteRequest(String),
}

impl TryFrom<&Vec<String>> for HttpRequest {
    type Error = HttpRequestParseError;

    fn try_from(s: &Vec<String>) -> Result<Self, HttpRequestParseError> {
        println!("{:?}", s);
        let req_line = s.get(0);

        println!("{:?}",req_line);

        if req_line.is_none() {
            return Err(HttpRequestParseError::IncompleteRequest("No request".into()));
        }

        let parts: Vec<&str> = req_line.unwrap().split(' ').collect();

        println!("{:?}", parts);

        if parts.len() != 3 {
            return Err(HttpRequestParseError::IncompleteRequest("Malformed or incomplete request".into()));
        }

        let method = HttpMethod::try_from(*parts.get(0).unwrap())?;
        let path = parts.get(1).unwrap().to_string();
        let version = HttpVersion::try_from(*parts.get(2).unwrap())?;

        Ok(HttpRequest {
            method,
            path,
            version,
            headers: vec![],
            body: "".to_string(),
        })
    }
}

// Response
// HTTP-Version Status-Code Reason-Phrase CRLF
// headers CRLF
// message-body
struct HttpResponse {
    version: HttpVersion,
    status_code: StatusCode,
    reason: String,
    headers: Vec<Header>,
    body: Option<String>,
}

impl HttpResponse {
    fn status_line(&self) -> String {
        format!(
            "{} {} {}",
            self.version.to_string(),
            self.status_code.to_code(),
            self.reason
        )
    }

    fn response_text(&self) -> String {
        let status_line = self.status_line();
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\r\n");
        format!(
            "{}\r\n{}\r\n\r\n{}",
            status_line,
            headers,
            self.body.clone().unwrap_or("".into())
        )
    }
}

#[derive(Debug)]
enum StatusCode {
    Ok,
    NotFound,
    BadRequest,
}

impl StatusCode {
    fn to_reason(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::NotFound => "Not Found",
        }
    }

    fn to_code(&self) -> u16 {
        match self {
            StatusCode::Ok => 200,
            StatusCode::BadRequest => 400,
            StatusCode::NotFound => 404,
        }
    }
}

struct HttpResponseBuilder {
    status_code: StatusCode,
    version: Option<HttpVersion>,
    reason: Option<String>,
    headers: Option<Vec<Header>>,
    body: Option<String>,
}

impl HttpResponseBuilder {
    pub fn new(status_code: StatusCode) -> HttpResponseBuilder {
        HttpResponseBuilder {
            status_code,
            version: None,
            reason: None,
            headers: None,
            body: None,
        }
    }

    pub fn set_version(mut self, v: Option<HttpVersion>) -> HttpResponseBuilder {
        self.version = v;
        self
    }

    pub fn set_reason(mut self, r: Option<String>) -> HttpResponseBuilder {
        self.reason = r;
        self
    }

    pub fn set_headers(mut self, hs: Option<Vec<Header>>) -> HttpResponseBuilder {
        self.headers = hs;
        self
    }

    pub fn add_header(mut self, header: Header) -> HttpResponseBuilder {
        let mut headers = match self.headers {
            Some(headers) => headers,
            None => vec![],
        };
        headers.push(header);
        self.headers = Some(headers);
        self
    }

    pub fn set_body(mut self, b: Option<String>) -> HttpResponseBuilder {
        self.body = b;
        self
    }

    pub fn build(self) -> HttpResponse {
        HttpResponse {
            version: self.version.unwrap_or(HttpVersion::Http11),
            reason: self.reason.unwrap_or(self.status_code.to_reason().into()),
            headers: self.headers.unwrap_or(vec![]),
            body: self.body,
            status_code: self.status_code,
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let http_request = HttpRequest::try_from(
        &buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<String>>(),
    );

    match http_request {
        Ok(req) => {
            println!("{:?}", req);

            if req.method == HttpMethod::GET && req.path == "/" {
                let contents = fs::read_to_string("index.html").unwrap();
                let content_length = contents.len();
                let response = HttpResponseBuilder::new(StatusCode::Ok)
                    .set_body(Some(contents))
                    .set_version(Some(req.version))
                    .add_header(("Content-Length".to_string(), content_length.to_string()))
                    .build()
                    .response_text();

                stream.write_all(response.as_bytes()).unwrap();
            } else {
                let contents = fs::read_to_string("404.html").unwrap();
                let content_length = contents.len();
                let response = HttpResponseBuilder::new(StatusCode::NotFound)
                    .set_body(Some(contents))
                    .set_version(Some(req.version))
                    .add_header(("Content-Length".to_string(), content_length.to_string()))
                    .build()
                    .response_text();

                stream.write_all(response.as_bytes()).unwrap();
            }
        },
        Err(e) => {
            println!("{:?}", e);
            let response = HttpResponseBuilder::new(StatusCode::BadRequest)
                .build()
                .response_text();

            stream.write_all(response.as_bytes()).unwrap();
        }
    }
}
