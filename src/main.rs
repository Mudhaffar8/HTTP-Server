mod threading;
mod tests;
mod constants;

extern crate flate2;

use std::{
    collections::HashMap, 
    fs, 
    io::{BufReader, Read, Write}, 
    net::{TcpListener, TcpStream}, 
    thread, 
    fmt
};

use crate::threading::ThreadPool;

use crate::constants::*;

use flate2::write::GzEncoder;
use flate2::Compression;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>
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
    body: Vec<u8>
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
            body: Vec::new()
        }
    }

    fn set_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.status_code = status_code;

        self
    }

    fn set_body(&mut self, body: Vec<u8>) -> &mut Self {
        self.body = body;

        self
    }

    fn set_header(&mut self, key: String, val: String) -> &mut Self {
        self.headers.insert(key, val);

        self
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut response = Vec::with_capacity(1024);

        response.extend_from_slice(format!("HTTP/1.1 {}\r\n\\", self.status_code).as_bytes());

        for (key, val) in self.headers.iter() {
            response.extend_from_slice(format!("{key}: {val}\r\n").as_bytes());
        }   

        response.extend_from_slice(b"\r\n");

        response.extend_from_slice(self.body.as_slice());   
          
        response 
    }
}

// Strictly for Debugging
impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "HTTP/1.1 {}\r\n\
            {}\
            \r\n",
            self.status_code.to_string(),
            self.headers.iter().map(|(s, k)| format!("{s}: {k}\r\n")).collect::<String>(),
        )
    }
}

impl HttpRequest {
    pub fn new_from_buffer(buffer: &[u8]) -> Result<Self, ParseError> {
        let request_string = String::from_utf8_lossy(&buffer);
        
        let mut request_lines = request_string.lines();

        let start_line_split = request_lines.next()
            .ok_or(ParseError::EmptyRequest)?
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

        let body = request_lines.next()
            .and_then(|s| 
                headers.get("Content-Length").and_then(|val| {
                    let len = val.parse::<usize>().unwrap_or_default();
                    Some(s[0..len].to_owned())
                })
            )
            .unwrap_or_default()
            .as_bytes()
            .to_vec();

        Ok(Self { 
            method,
            path,
            headers, 
            body
        })
    }
}

// Strictly for Debugging
impl fmt::Display for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{} {} HTTP/1.1\r\n\
            {}\
            \r\n",
            self.method,
            self.path,
            self.headers.iter().map(|(s, k)| format!("{s}: {k}\r\n")).collect::<String>(),
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
                let contents = fs::read(PATH_TO_HOMEPAGE).unwrap_or_default();
                let len = contents.len();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Length".to_owned(), len.to_string())
                    .set_body(contents);
            },

            // For testing concurrency
            "/sleep" => { 
                let content = fs::read(PATH_TO_HOMEPAGE).unwrap_or_default();
                let len = content.len();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Length".to_owned(), len.to_string())
                    .set_body(content);

                thread::sleep(std::time::Duration::from_secs(5));
            },

            path if path.starts_with("/echo/") => {
                let echo_string = path.strip_prefix("/echo/").unwrap_or("");

                // TODO: Move Compression code into set_body method
                if let Some(val) = request.headers.get("Accept-Encoding") {
                    for compression in val.split(", ") {
                        if compression == "gzip" {
                            resp.set_header("Content-Encoding".to_owned(), "gzip".to_owned());
                        }
                    }
                }

                let mut encoder: GzEncoder<Vec<u8>> = GzEncoder::new(Vec::new(), Compression::default());

                encoder.write_all(echo_string.as_bytes())?;
                let compressed_data = encoder.finish()?;

                //println!("{:?}", compressed_data);

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type".to_owned(), "text/plain".to_owned())
                    .set_header("Content-Encoding".to_owned(), "gzip".to_owned())
                    .set_header("Content-Length".to_owned(), compressed_data.len().to_string())
                    .set_body(compressed_data);

            },

            path if path.starts_with("/files/") => {
                let file_path = format!("/tmp/{}", path.strip_prefix("/files/").unwrap_or_default());

                match fs::read(file_path.as_str()){
                    Ok(contents) => {
                        resp
                            .set_status_code(StatusCode::Ok)
                            .set_header("Content-Type".to_owned(), "application/octet-stream".to_owned())
                            .set_header("Content-Length".to_owned(), contents.len().to_string())
                            .set_body(contents); 
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        resp.set_status_code(StatusCode::NotFound);
                    } 
                }
            },

            path if path.starts_with("/user-agent") => {
                let user_agent: Vec<u8> = request.headers.get("User-Agent")
                    .unwrap_or(&"".to_owned()) 
                    .as_bytes()
                    .to_vec();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type".to_owned(), "text/plain".to_owned())
                    .set_header("Content-Length".to_owned(), user_agent.len().to_string())
                    .set_body(user_agent);
            },

            _ => { 
                resp.set_status_code(StatusCode::NotImplemented); 
                //println!("{}", request.path);
            }
        }    
    } else if request.method == "POST" {
        match request.path.as_str() {
            path if path.starts_with("/files/") => {
                let file_name= format!("/tmp/{}", path.strip_prefix("/files/").unwrap_or_default());

                println!("{}", file_name);

                match fs::write(file_name, request.body.as_slice()) {
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

    stream.write_all(&resp.as_bytes())?;
    stream.flush()?;

    println!("{}", resp);

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
                        println!("Unable to Process Client Request: {}", e);
                    }
                });
            },               
            Err(e) => { println!("Error: {:?}", e); }
        }
    }
}