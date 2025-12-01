#[cfg(test)]
mod tests {
    use crate::HttpRequest;

    #[test]
    fn parse_check_status_line() {
        let request = b"GET / HTTP/1.1\r\n\r\n";
        let req = HttpRequest::new_from_buffer(request).unwrap();

        println!("{}", req);

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/");
        assert_eq!(req.body, b"");
    }

    #[test]
    fn parse_check_w_headers() {
        let request = b"GET / HTTP/1.1\r\n\
                Host: Moody.com\r\n\
                Content-Length: 10\r\n\
                Content-Type: text/plain\r\n\
                \r\n";
        let req = HttpRequest::new_from_buffer(request).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/");

        // Check Headers
        assert_eq!(req.headers.get("Host").unwrap().as_str(), 
            "Moody.com"
        );

        assert_eq!(req.headers.get("Content-Length").unwrap().as_str(), 
            "10"
        );

        assert_eq!(req.headers.get("Content-Type").unwrap().as_str(), 
            "text/plain"
        );
    }

    #[test]
    fn parse_check_w_body() {
        let request = b"GET /echo/banana HTTP/1.1\r\n\
            Host: localhost:4221\r\n\
            Accept-Encoding: gzip, deflate, br, zstd\r\n\
            Content-Length: 6\r\n\
            \r\n\
            banana";

        let req = HttpRequest::new_from_buffer(request).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/echo/banana");

        // Check Headers
        assert_eq!(req.headers.get("Host").unwrap().as_str(), 
            "localhost:4221"
        );

        assert_eq!(req.headers.get("Accept-Encoding").unwrap().as_str(), 
            "gzip, deflate, br, zstd"
        );

        assert_eq!(req.body.as_slice(), b"banana");
    }

    #[test]
    fn gzip_test() {

    }
}