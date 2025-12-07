use std::{
    collections::HashMap,
    io::Write,
    fmt
};

use crate::request::HttpRequest;

/// HTTP status codes supported by this server.
/// 
/// Values correspond to their numeric meanings.
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum StatusCode {
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

/// Represents an HTTP response that will be returned to the client.
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>
}

/// **Strictly for Debugging**: Serializes HTTP request status line and headers into HTTP/1.1 format. 
/// Not suitable for network transmissions as body is not included and body may contain non-UTF-8 data.
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

impl HttpResponse {
    /// Status defaults to 404.
    pub fn new() -> Self {
        Self {
            status_code: StatusCode::NotFound,
            headers: HashMap::new(),
            body: Vec::new()
        }
    }

    pub fn set_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.status_code = status_code;

        self
    }

    pub fn set_header(&mut self, key: String, val: String) -> &mut Self {
        self.headers.insert(key, val);

        self
    }

    /// Sets HTTP response body and automatically applies gzip compression.
    /// only if client indicates support via `Accept-Encoding` Header.
    /// 
    /// Additionally updates Content-Encoding and/or Content-Length Headers as appropriate.
    pub fn set_body(&mut self, contents: Vec<u8>, request: &HttpRequest) -> Result<(), std::io::Error> {
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
    pub fn as_bytes(&self) -> Vec<u8> {
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
