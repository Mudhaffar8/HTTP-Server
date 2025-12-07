use std::{
    collections::HashMap,
    fmt
};

/// Represents errors that can occur while parsing an HTTP request.
/// 
/// For debugging purposes.
#[derive(Debug, Clone, Copy)]
pub enum ParseError {
    EmptyRequest,
    EmptyMethod,
    EmptyPath,
    EmptyHttpVersion, 
    InvalidHeader
}

pub enum HttpMethod {
    GET,
    POST,
    HEAD
}

/// Represents a parsed HTTP Request received from a client.
#[derive(Debug)]
pub struct HttpRequest {
    /// Currently supports GET and POST methods.
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>
}

/// **Strictly for Debugging**: Serializes HTTP Request status line and headers into HTTP/1.1 format. 
/// Not suitable for network transmissions as body is not included as body may contain non-UTF-8 data.
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

/*
- Add new_404 method for more clarity?
- Add methods for configuring common HTTP responses:
    - CSS, HTML, Images, JSON Data
*/
impl HttpRequest {
    /// **Strictly for testing**: Creates minimal `HttpRequest`.
    /// TODO?: Refactor method var into enum.
    #[cfg(test)]
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_owned(),
            path: path.to_owned(),
            headers: HashMap::new(),
            body: Vec::new()
        }
    }

    /// **Strictly for testing**: Adds or updates HTTP Response Headers.
    #[cfg(test)]
    pub fn set_header(&mut self, key: String, val: String) -> &mut Self {
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
    pub fn new_from_buffer(buffer: &[u8]) -> Result<Self, ParseError> {
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