mod constants;
mod response;
mod request;
mod router;
mod tests;
mod threading;
mod utils;

use std::{
    fs, 
    io::{BufReader, Read, Write}, 
    net::{TcpListener, TcpStream}, 
    thread
};

extern crate flate2;

use crate::constants::*;
use crate::response::*;
use crate::request::*;
use crate::threading::ThreadPool;
use crate::utils::*;

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