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

/// Represents a parsed HTTP Request received from a client.
#[derive(Debug)]
struct HttpRequest {
    /// Currently supports GET and POST methods.
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>
}

/// Represents errors that can occur while parsing an HTTP request.
/// 
/// For debugging purposes.
#[derive(Debug, Clone, Copy)]
enum ParseError {
    EmptyRequest,
    EmptyMethod,
    EmptyPath,
    EmptyHttpVersion, 
    InvalidHeader
}

/// Represents an HTTP response that will be returned to the client.
#[derive(Debug)]
struct HttpResponse {
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>
}

/// HTTP status codes supported by this server.
/// 
/// Values correspond to their numeric meanings.
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
enum StatusCode {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    NotFound = 404,
    InternalServerError  = 500
}

/// Used for serializing a status code.
/// 
/// Outputs status number and status message as part of HTTP status line.
impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", *self as u32, match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::NotFound => "Not Found",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::InternalServerError => "Internal Server Error"
        })
    }
}

impl HttpResponse {
    /// Status defaults to 404.
    fn new() -> Self {
        Self {
            status_code: StatusCode::NotFound,
            headers: HashMap::new(),
            body: Vec::new()
        }
    }

    fn set_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.status_code = status_code;

        self
    }

    fn set_header(&mut self, key: String, val: String) -> &mut Self {
        self.headers.insert(key, val);

        self
    }

    /// Sets HTTP response body and automatically applies gzip compression.
    /// only if client indicates support via `Accept-Encoding` Header.
    /// 
    /// Additionally updates Content-Encoding and/or Content-Length Headers as appropriate.
    fn set_body(&mut self, contents: Vec<u8>, request: &HttpRequest) -> Result<(), std::io::Error> {
        if let Some(encoding) = request.headers.get("Accept-Encoding") {
            let is_gzip_enabled = encoding
                .split(", ")
                .any(|compression_mode| compression_mode == "gzip");

            if is_gzip_enabled {
                return self.compress_body(contents);
            }
        }

        self.set_header("Content-Length".to_owned(), contents.len().to_string());
        self.body = contents;

        Ok(())
    }

    /// Performs gzip compression and updates body.
    /// 
    /// Additionally updates Content-Encoding and Content-Length Headers.
    /// 
    /// Used internally by `set_body()`.
    fn compress_body(&mut self, data: Vec<u8>) -> Result<(), std::io::Error> { 
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder: GzEncoder<Vec<u8>> = GzEncoder::new(Vec::new(), Compression::default());

        encoder.write_all(data.as_slice())?;
        let compressed_data = encoder.finish()?;

        self.set_header("Content-Encoding".to_owned(), "gzip".to_owned());
        self.set_header("Content-Length".to_owned(), compressed_data.len().to_string());

        self.body = compressed_data;

        Ok(())
    }


    /// Serializes `HttpResponse` into HTTP/1.1 response byte buffer.
    fn as_bytes(&self) -> Vec<u8> {
        let mut response = Vec::with_capacity(self.body.len() + 250);

        response.extend_from_slice(format!("HTTP/1.1 {}\r\n", self.status_code).as_bytes());

        for (key, val) in self.headers.iter() {
            response.extend_from_slice(key.as_bytes());
            response.extend_from_slice(b": ");
            response.extend_from_slice(val.as_bytes());
            response.extend_from_slice(b"\r\n");
        }   

        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(self.body.as_slice());   
          
        response 
    }
}

/// **Strictly for Debugging**: Serializes Request status line and header into HTTP/1.1 format. 
/// May not be suitable for network transmissions if body contains non-UTF-8 data.
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
    /// **Strictly for testing**: Creates minimal `HttpRequest`.
    /// TODO?: Refactor method var into enum.
    #[cfg(test)]
    fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_owned(),
            path: path.to_owned(),
            headers: HashMap::new(),
            body: Vec::new()
        }
    }

    /// **Strictly for testing**: Adds or updates HTTP Response Headers.
    #[cfg(test)]
    fn set_header(&mut self, key: String, val: String) -> &mut Self {
        self.headers.insert(key, val);

        self
    }

    /// Creates a new `HttpRequest` by parsing a raw byte buffer.
    /// 
    /// # Returns
    /// - `Ok(HttpRequest)` if buffer is successfully parsed.
    /// - `Err(ParseError)` if request is malformed or is missing required components.
    /// 
    /// Note:
    /// - Buffer must contain valid UTF-8 encoded headers and status line.
    fn new_from_buffer(buffer: &[u8]) -> Result<Self, ParseError> {
        let request_string = String::from_utf8_lossy(&buffer);
        
        let mut request_lines = request_string.lines();

        // Ensure string exists
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

        // Parse all Headers
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

/// **Strictly for Debugging**: Serializes HTTP Request status line and headers into HTTP/1.1 format. 
/// Not be suitable for network transmissions as body is not included and may contain non-UTF-8 data.
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

/// Checks whether a file system path is safe to serve
/// 
/// Path is considered safe iff it:
/// - Does not contain `".."` components (prevents parent directory traversal)
/// - Does not start with an absolute root
/// - Does not contain prefix components (such as Windows `C:\` drives)
fn is_path_safe(path : &str) -> bool
{
    use std::path::{Path, Component};

    let path = Path::new(path);

   for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,

            Component::CurDir | Component::Normal(_) => continue
        }
   }

    true
}

/// Handles a single client connection.
/// 
/// This function will:
/// 1. Read incoming requests from a client.
/// 2. Parse requests into an `HttpRequest`.
/// 3. If parsed with no errors, generate appropriate `HttpResponse`.
/// 4. Route request to handlers based on request path.
/// 5. Send serialized response back to client.
/// 
/// Note:
/// - Parsing errors result in connection being closed immediately.
fn handle_client(mut stream: TcpStream) -> Result<(), std::io::Error> {
    println!("Incoming Connection: {:?}", stream.peer_addr());

    let mut buffer = vec![0u8; 1024];
    let mut reader = BufReader::new(&stream);

    reader.read(&mut buffer)?;

    let mut resp = HttpResponse::new();

    // Return early if Parsing Error
    let Ok(request) = HttpRequest::new_from_buffer(&buffer) else {
        resp.set_status_code(StatusCode::BadRequest);
        println!("{}", resp);

        stream.write_all(&resp.as_bytes())?;
        stream.flush()?;

        return Ok(());
    };


    if request.method == "GET" {
        match request.path.as_str() {
            // Homepage
            "/" => {
                if let Ok(contents) = fs::read(PATH_TO_HOMEPAGE) {
                    resp
                        .set_status_code(StatusCode::Ok)
                        .set_header("Content-Type".to_owned(), "text/html".to_owned())
                        .set_body(contents, &request)?;
                }
            },

            // Handles Requests for CSS files
            path if path.starts_with("/css/") => {
                if let Some(css_file_name) = path.strip_prefix("/css/") {
                    if is_path_safe(css_file_name) {

                        if let Ok(contents) = fs::read(PATH_TO_CSS.to_owned() + &css_file_name) {                 
                            resp
                                .set_status_code(StatusCode::Ok)
                                .set_header("Content-Type".to_owned(), "text/css".to_owned())
                                .set_body(contents, &request)?;
                        }
                    }
                }
            },

            // Handles Requests for Image Files (SVG and JPG supported)
            path if path.starts_with("/images/") => {
                if let Some(img_file_name) = path.strip_prefix("/images/") {
                    if is_path_safe(img_file_name) {

                        // Accounting for different image file types
                        if img_file_name.ends_with(".svg") { resp.set_header("Content-Type".to_owned(), "image/svg+xml".to_owned()); }
                        else if img_file_name.ends_with(".jpg") { resp.set_header("Content-Type".to_owned(), "image/jpg".to_owned()); }

                        if let Ok(contents) = fs::read(PATH_TO_IMAGES.to_owned() + &img_file_name) {
                            resp
                                .set_status_code(StatusCode::Ok)
                                .set_body(contents, &request)?;
                        }
                   }
                }
            },

            // For testing concurrency
            "/sleep" => { 
                if let Ok(content) = fs::read(PATH_TO_HOMEPAGE) {
                
                    resp
                        .set_status_code(StatusCode::Ok)
                        .set_body(content, &request)?;

                    thread::sleep(std::time::Duration::from_secs(5));
                }
            },

            path if path.starts_with("/echo/") => {
                // Will never default
                let echo_string = path.strip_prefix("/echo/").unwrap_or("");

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_body(echo_string.as_bytes().to_vec(), &request)?;

            },

            path if path.starts_with("/files/") => {
                // Will never default
                if let Some(path) = path.strip_prefix("/files/") {
                    let file_path = format!("/tmp/{}", path.strip_prefix("/files/").unwrap_or_default());

                    if is_path_safe(file_path.as_str()) {
                        match fs::read(file_path.as_str()) {
                            Ok(contents) => {
                                resp
                                    .set_status_code(StatusCode::Ok)
                                    .set_header("Content-Type".to_owned(), "application/octet-stream".to_owned())
                                    .set_body(contents, &request)?; 
                            },
                            Err(e) =>  { println!("Error: {}", e); }
                        }
                    }
                }
            },

            path if path.starts_with("/user-agent") => {
                let user_agent: Vec<u8> = request.headers.get("User-Agent")
                    .unwrap_or(&"No User Agent".to_owned()) 
                    .as_bytes()
                    .to_vec();

                resp
                    .set_status_code(StatusCode::Ok)
                    .set_header("Content-Type".to_owned(), "text/plain".to_owned())
                    .set_body(user_agent, &request)?;
            },

            _ => { 
                resp.set_status_code(StatusCode::NotFound); 
                //println!("{}", request.path);
            }
        }    
    } else if request.method == "POST" {
        match request.path.as_str() {

            // Writes content in request body to file
            path if path.starts_with("/files/") => {
                let file_name= format!("/tmp/{}", path.strip_prefix("/files/").unwrap_or_default());

                //println!("{}", file_name);

                match fs::write(file_name, request.body.as_slice()) {
                    Ok(_) => { resp.set_status_code(StatusCode::Created); },

                    Err(e) => {
                        println!("Error: {}", e);
                        resp.set_status_code(StatusCode::InternalServerError); 
                    }
                }
            },

            _ => { 
                //resp.set_status_code(StatusCode::NotFound); 
                //println!("{}", request.path);
            }
        }
    }

    stream.write_all(&resp.as_bytes())?;
    stream.flush()?;

    println!("{}", resp);

    Ok(())
}


fn main() {
    let listener = TcpListener::bind(ADDRESS)
        .expect("Failed to bind TCP listener. Address may be in use or invalid.");

    let pool = ThreadPool::new(NUM_OF_THREADS);
    
    for stream in listener.incoming() {
        match stream {
            Ok(s) => { 
                pool.execute(|| {
                    handle_client(s).unwrap_or_else(|err|
                        println!("Error processing client request: {}", err)
                    );
                });
            },               
            Err(e) => { println!("Error: {:?}", e); }
        }
    }
}