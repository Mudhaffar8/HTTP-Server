use std::{
    collections::HashMap, fs, hash::Hash, io::{BufReader, Read, Write}, net::{TcpListener, TcpStream}, thread, time::Duration
};

mod threading;

use crate::threading::ThreadPool;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String
}

#[allow(dead_code)]
struct HttpResponse {
    status_code: String,
    status_msg : String,
    headers: HashMap<String, String>,
    body: String
}

impl HttpRequest {
    pub fn new_from_vec(buffer: Vec<u8>) -> Self {
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
            let len = headers.get("Content-Length").unwrap().parse::<i32>().unwrap() as usize;
            s[0..len].to_owned()
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

    let mut buffer = vec![0u8; 512];
    let mut reader = BufReader::new(&stream);

    reader.read(&mut buffer).unwrap();

    let request = HttpRequest::new_from_vec(buffer);


    if request.method == "GET" {
        match request.path.as_str() {
            "/" => {
                let content = fs::read_to_string("./src/main.html").unwrap();

                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Length: {}\r\n\r\n\
                    {content}\r\n",
                    content.len()
                );

                stream.write_all(response.as_bytes()).unwrap() 
            },

            // Testing multiple threads
            "/sleep" => { 
                let content = fs::read_to_string("./src/main.html").unwrap();

                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Length: {}\r\n\r\n\
                    {content}\r\n",
                    content.len()
                );

                thread::sleep(Duration::from_secs(5));

                stream.write_all(response.as_bytes()).unwrap();
            },

            path if path.starts_with("/echo/") => {
                let echo_string = path.strip_prefix("/echo/").unwrap_or_else(|| "");

                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: {}\r\n\r\n\
                    {echo_string}\r\n",
                    echo_string.len()
                );

                stream.write_all(response.as_bytes()).unwrap();
            },

            path if path.starts_with("/files/") => {
                let file_path = format!("/{}", path.strip_prefix("/files/").unwrap_or_else(|| ""));

                match fs::read(file_path.as_str()){
                    Ok(contents) => {
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: application/octet-stream\r\n\
                            Content-Length: {}\r\n\r\n\
                            {:?}\r\n",
                            contents.len(),
                            contents
                        );

                        stream.write_all(response.as_bytes()).unwrap();
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
                    } 
                }
            },

            path if path.starts_with("/user-agent") => {
                let user_agent = request.headers.get("User-Agent").unwrap().as_str();

                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: {}\r\n\r\n\
                    {user_agent}\r\n",
                    user_agent.len()
                );

                stream.write_all(response.as_bytes()).unwrap();
            },

            _ => stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap()
        }    
    } else if request.method == "POST" {
        match request.path.as_str() {
            path if path.starts_with("/files/") => {
                let file_name= format!("/{}", path.strip_prefix("/files/").unwrap_or_else(|| ""));

                match fs::write(file_name, request.body.as_bytes()) {
                    Ok(_) => stream.write_all(b"HTTP/1.1 201 Created\r\n\r\n").unwrap(),
                    Err(e) => {
                        println!("Error: {}", e);
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
                    }
                }

            },
            _ => { 
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
            }
        }
    }
    
    stream.flush().unwrap();
}


fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let pool = ThreadPool::new(4);
    
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