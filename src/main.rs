use std::{
    collections::HashMap, 
    fs, 
    io::{BufReader, Read, Write}, 
    net::{TcpListener, TcpStream}, 
    thread, 
    fmt
};


mod threading;
mod tests;

use crate::threading::ThreadPool;

const ADDRESS: &'static str = "127.0.0.1:4221";
const NUM_OF_THREADS: usize = 4;

const VALID_COMPRESSION_MODES: [&'static str; 1] = ["gzip"];

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String
}

#[allow(dead_code)]
#[derive(Debug)]
struct HttpResponse {
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: String
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum StatusCode {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    NotFound = 404,
    InternalServerError  = 500,
    NotImplemented = 501,
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", *self as u32, match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::NotFound => "Not Found",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::InternalServerError => "Internal Server Error"
        })
    }
}

impl HttpResponse {
    fn new() -> Self {
        Self {
            status_code: StatusCode::Ok,
            headers: HashMap::new(),
            body: String::new()
        }
    }

    fn set_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.status_code = status_code;

        self
    }

    fn set_body(&mut self, body: String) -> &mut Self {
        self.body = body;

        self
    }

    fn set_header(&mut self, key: &str, val: &str) -> &mut Self {
        self.headers.insert(key.to_owned(), val.to_owned());

        self
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "HTTP/1.1 {}\r\n\
            {}\
            \r\n\
            {}",
            self.status_code.to_string(),
            self.headers.iter().map(|(s, k)| format!("{s}: {k}\r\n")).collect::<String>(),
            self.body,
        )
    }
}

impl HttpRequest {
    pub fn new_from_buffer(buffer: &[u8]) -> Self {
        let request_string = String::from_utf8_lossy(&buffer);
        
        let mut request_lines = request_string.lines();

        let mut start_line_split = request_lines.next().unwrap().split_whitespace();

        let method = start_line_split.next().unwrap().to_owned();
        let path = start_line_split.next().unwrap().to_owned();

        let mut headers: HashMap<String, String> = HashMap::new();

        for line in request_lines.by_ref() {
            if line.is_empty() {
                break;
            }
            let header_split = line.split(": ").collect::<Vec<&str>>();
            headers.insert(header_split[0].to_owned(), header_split[1].to_owned());
        }

        let body = if let Some(s) = request_lines.next() { 
            match headers.get("Content-Length") {
                Some(val) => {
                    let len = val.parse::<usize>().unwrap();
                    s[0..len].to_owned()
                },
                None => "".to_owned()
            } 
        } else { 
            "".to_owned() 
        };

        Self { 
            method,
            path,
            headers, 
            body
        }
    }
}


fn handle_client(mut stream: TcpStream) {
    println!("Incoming Connection: {:?}", stream.peer_addr());

    let mut buffer = [0u8; 1024];
    let mut reader = BufReader::new(&stream);

    reader.read(&mut buffer).unwrap();


    let request = HttpRequest::new_from_buffer(&buffer);

    let mut resp = HttpResponse::new();

    if request.method == "GET" {
        match request.path.as_str() {
            "/" => {
                let contents = fs::read_to_string("./src/main.html").unwrap();
                let len = contents.len().to_string();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Length", len.as_str())
                    .set_body(contents);
            },

            // For testing concurrency
            "/sleep" => { 
                let content = fs::read_to_string("./src/main.html").unwrap();
                let len = content.len().to_string();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Length", len.as_str())
                    .set_body(content);

                thread::sleep(std::time::Duration::from_secs(5));
            },

            path if path.starts_with("/echo/") => {
                let echo_string = path.strip_prefix("/echo/").unwrap_or_else(|| "");

                if let Some(val) = request.headers.get("Accept-Encoding") {
                    for compression in val.split(", ") {
                        if compression == "gzip" {
                            // Actually implement compression modes :)
                            resp.set_header("Content-Encoding", "gzip");
                        }
                    }
                }

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type", "text/plain")
                    .set_header("Content-Length", echo_string.len().to_string().as_str())
                    .set_body(echo_string.to_string());

            },

            path if path.starts_with("/files/") => {
                let file_path = format!("/{}", path.strip_prefix("/files/").unwrap_or_else(|| ""));

                match fs::read(file_path.as_str()){
                    Ok(contents) => {
                        resp
                            .set_status_code(StatusCode::Ok)
                            .set_header("Content-Type", "application/octet-stream")
                            .set_header("", contents.len().to_string().as_str())
                            .set_body(String::from_utf8(contents).unwrap()); // FIX ME
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        resp.set_status_code(StatusCode::NotFound);
                    } 
                }
            },

            path if path.starts_with("/user-agent") => {
                let user_agent = request.headers.get("User-Agent").unwrap();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type", "text/plain")
                    .set_header("Content-Length", user_agent.len().to_string().as_str())
                    .set_body(user_agent.clone());
            },

            _ => { resp.set_status_code(StatusCode::NotImplemented); }
        }    
    } else if request.method == "POST" {
        match request.path.as_str() {
            path if path.starts_with("/files/") => {
                let file_name= format!("/{}", path.strip_prefix("/files/").unwrap_or_else(|| ""));

                match fs::write(file_name, request.body.as_bytes()) {
                    Ok(_) => { resp.set_status_code(StatusCode::Created); },

                    Err(e) => {
                        println!("Error: {}", e);
                        resp.set_status_code(StatusCode::InternalServerError); 
                    }
                }
            },
            _ => { resp.set_status_code(StatusCode::NotFound); }
        }
    } else {
        resp.set_status_code(StatusCode::NotImplemented);
    }

    println!("{:?}", resp);

    stream.write_all(resp.to_string().as_bytes()).unwrap();
    stream.flush().unwrap();
}


fn main() {
    let listener = TcpListener::bind(ADDRESS).unwrap();

    let pool = ThreadPool::new(NUM_OF_THREADS);
    
    for stream in listener.incoming() {
        match stream {
            Ok(s) => { 
                pool.execute(|| {
                    handle_client(s);
                });
            },              
            Err(e) => { println!("Error: {:?}", e); }
        }
    }
}