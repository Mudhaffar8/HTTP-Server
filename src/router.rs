use std::{
    collections::HashMap,
    io::{BufReader, Read, Write}, 
    net::TcpStream, 
};

use crate::response::{HttpResponse, StatusCode};
use crate::request::HttpRequest;

type RouterFunc = Box<dyn Fn(&HttpRequest, &mut HttpResponse) -> Result<(), std::io::Error>>;

pub struct Router {
    routes : HashMap<&'static str, RouterFunc>,
}

/*
- Have seperate data structures for handling exact and prefix routes?
*/
impl Router {
    /// Creates a new and empty `Router`.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new()
        }
    }

    /// Registers a new route and its corresponding handler function.
    pub fn add_route(&mut self, path : &'static str, func : RouterFunc) {
        self.routes.insert(path, Box::new(func));
    }

    /// Sends an `Httpesponse` over the provided `TcpStream`
    fn send_response(&mut self, mut stream: TcpStream, resp: HttpResponse) -> Result<(), std::io::Error> {
        stream.write_all(&resp.as_bytes())?;
        stream.flush()?;

        println!("{}", resp);

        Ok(())
    }

    // TODO
    pub fn handle_request(&mut self, mut stream : TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = vec![0u8; 1024];

        let mut reader = BufReader::new(&stream);
        reader.read(&mut buffer)?;

        // 404 by default
        let mut resp = HttpResponse::new();

        // Return early if Parsing Error
        let Ok(req) = HttpRequest::new_from_buffer(&buffer) else {
            resp.set_status_code(StatusCode::BadRequest);
            
            return self.send_response(stream, resp);
        };

        if let Some(func) = self.routes.get(req.path.as_str()) {
            let _ = func(&req, &mut resp)?;
        }
        
        self.send_response(stream, resp)
    } 
}