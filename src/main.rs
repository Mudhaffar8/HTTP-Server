use std::{
    collections::HashMap, 
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    fs,
    thread,
    time::Duration
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

impl HttpRequest {
    pub fn new_from_vec(request_lines: &Vec<String>) -> HttpRequest {
        let mut start_line_split = request_lines[0].split_whitespace();

        let method = start_line_split.next().unwrap();
        let path = start_line_split.next().unwrap();

        let mut headers: HashMap<String, String> = HashMap::new();

        for line in request_lines[1..].iter() {
            if line.is_empty() {
                break;
            }
            let header_split = line.split(": ").collect::<Vec<&str>>();

            headers.insert(header_split[0].to_owned(), header_split[1].to_owned());
        }

        HttpRequest { 
            method: method.to_owned(), 
            path: path.to_owned(), 
            headers, 
            body: "".to_owned() 
        }
    }
}


fn handle_client(mut stream: TcpStream) {
    println!("Incoming Connection: {:?}", stream.peer_addr());

    let reader = BufReader::new(&stream);

    let request_lines = reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();


    let request = HttpRequest::new_from_vec(&request_lines);

    println!("Request: {:#?}", request);


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

        "/sleep" => { 
            thread::sleep(Duration::from_secs(5));
            stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
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