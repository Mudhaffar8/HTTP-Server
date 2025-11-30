mod threading;
mod tests;
mod constants;

extern crate flate2;
use crate::threading::ThreadPool;

use crate::constants::*;

use std::{
    collections::HashMap, 
    fs, 
    io::{BufReader, Read, Write, Error, ErrorKind}, 
    net::{TcpListener, TcpStream}, 
    thread, 
    fmt
};

use flate2::write::GzEncoder;
use flate2::Compression;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String
}

#[derive(Debug, Clone, Copy)]
enum ParseError {
    EmptyRequest,
    EmptyMethod,
    EmptyPath,
    EmptyHttpVersion, 
    InvalidHeader
}

#[derive(Debug)]
struct HttpResponse {
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: String
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
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
    pub fn new_from_buffer(buffer: &[u8]) -> Result<Self, ParseError> {
        let request_string = String::from_utf8_lossy(&buffer);
        
        let mut request_lines = request_string.lines();

        let start_line_split = request_lines.next()
            .ok_or( ParseError::EmptyRequest)?
            .to_owned();

        let mut parts = start_line_split.split_whitespace();

        // Check Method exists in string
        let method = parts.next()
            .ok_or(ParseError::EmptyMethod)?
            .to_owned();

            
        // Check path exists in string
        let path = parts.next()    
            .ok_or(ParseError::EmptyPath)?
            .to_owned();

        // Check if HTTP Version exists
        let _http_version = parts.next().ok_or(ParseError::EmptyHttpVersion)?;
        
        let mut headers: HashMap<String, String> = HashMap::new();

        // Parse all Headers and add to Hashmap
        for line in request_lines.by_ref() {
            if line.is_empty() {
                break;
            }

            let mut header_split = line.split(": ");

            let (Some(key), Some(val)) = (header_split.next(), header_split.next()) else {
                // Make this continue instead?
                return Err(ParseError::InvalidHeader);
            };
            
            headers.insert(key.to_owned(), val.to_owned());
        }

        let body = if let Some(s) = request_lines.next() { 
            match headers.get("Content-Length") {
                Some(val) => {
                    let len = val.parse::<usize>().unwrap_or_default();
                    s[0..len].to_owned()
                },
                None => "".to_owned()
            } 
        } else { 
            "".to_owned() 
        };

        Ok(Self { 
            method,
            path,
            headers, 
            body
        })
    }
}

impl fmt::Display for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{} {} HTTP/1.1\r\n\
            {}\
            \r\n\
            {}",
            self.method,
            self.path,
            self.headers.iter().map(|(s, k)| format!("{s}: {k}\r\n")).collect::<String>(),
            self.body,
        )
    }
}

fn handle_client(mut stream: TcpStream) -> Result<(), std::io::Error> {
    println!("Incoming Connection: {:?}", stream.peer_addr());

    let mut buffer = [0u8; 1024];
    let mut reader = BufReader::new(&stream);

    reader.read(&mut buffer)?;

    let mut resp = HttpResponse::new();

    // Return early if Parsing Error
    let Ok(request) = HttpRequest::new_from_buffer(&buffer) else {
        resp.set_status_code(StatusCode::BadRequest);
        println!("{:?}", resp);

        stream.write_all(resp.to_string().as_bytes())?;
        stream.flush()?;

        return Ok(());
    };

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

                let mut buffer: Vec<u8> = Vec::new();

                let mut encoder = GzEncoder::new(&mut buffer, Compression::default());

                let mut cursor = std::io::Cursor::new(echo_string.as_bytes());
                std::io::copy(&mut cursor, &mut encoder).unwrap();
                let compressed = encoder.finish().unwrap();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type", "text/plain")
                    .set_header("Content-Encoding", "gzip")
                    .set_header("Content-Length", &compressed.len().to_string())
                    .set_body(String::from_utf8_lossy(&compressed).to_string());

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
        resp.set_status_code(StatusCode::InternalServerError);
    }

    println!("{:?}", resp);

    stream.write_all(resp.to_string().as_bytes())?;
    stream.flush()?;

    Ok(())
}


fn main() {
    let listener = TcpListener::bind(ADDRESS).unwrap();

    let pool = ThreadPool::new(NUM_OF_THREADS);
    
    for stream in listener.incoming() {
        match stream {
            Ok(s) => { 
                pool.execute(|| {
                    if let Err(e) = handle_client(s) {
                        println!("{}", e);
                    }
                });
            },               
            Err(e) => { println!("Error: {:?}", e); }
        }
    }
}