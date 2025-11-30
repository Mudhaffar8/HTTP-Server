#[cfg(test)]
mod tests {
    use crate::HttpRequest;

    #[test]
    fn parse_check() {
        let request = b"GET / HTTP/1.1\r\n\r\n";
        let req = HttpRequest::new_from_buffer(request).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/");
        assert_eq!(req.body, "");
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
}