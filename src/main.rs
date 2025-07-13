use std::{
    collections::HashMap, 
    io::{BufReader, Read, Write}, 
    net::{TcpListener, TcpStream}
};

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String
}

impl HttpRequest {
    pub fn new_from_buffer(buffer: &[u8]) -> HttpRequest {
        let request = String::from_utf8_lossy(&buffer);

        let mut request_lines = request.lines();
        let mut start_line_split = request_lines.next().unwrap().split_whitespace();

        let method = start_line_split.next().unwrap();
        let path = start_line_split.next().unwrap();

        let mut headers = HashMap::new();

        for line in request_lines {
            if line.is_empty() {
                break;
            }
            let header_split = line.split(": ").collect::<Vec<&str>>();

            if header_split.len() == 2 {
                headers.insert(header_split[0].to_owned(), header_split[1].to_owned());
            }
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

    let mut buffer = [0u8; 512];
    let mut reader = BufReader::new(&stream);

    reader.read(&mut buffer).unwrap();

    let request = HttpRequest::new_from_buffer(&buffer);

    println!("{:#?}", request);


    match request.path.as_str() {
        "/" => stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap(),

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
    
    for stream in listener.incoming() {
        match stream {
            Ok(s) => { 
                handle_client(s);
            },              
            Err(e) => { println!("Error: {:?}", e); }
        }
    }
}