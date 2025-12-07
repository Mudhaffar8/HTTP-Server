#[cfg(test)]
mod tests {
    use crate::request::*;
    use crate::response::*;
    use crate::utils::is_path_safe;

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
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let body = b"supercalifragilisticexpialidocious".to_vec();

        let mut resp = HttpResponse::new();
        let mut req = HttpRequest::new("GET", "/");

        let mut encoder: GzEncoder<Vec<u8>> = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body.as_slice()).expect("GZIP Error");
        let compressed_data = encoder.finish().expect("GZIP Error");

        req.set_header("Accept-Encoding".to_owned(), "gzip, deflate, br, zstd".to_owned());
        resp.set_body(body, &req).expect("GZIP Error");

        assert_eq!(resp.body, compressed_data);
    }

    #[test]
    fn no_compression_body() {
        let body = b"supercalifragilisticexpialidocious";

        let mut resp = HttpResponse::new();
        let mut req = HttpRequest::new("GET", "/");

        req.set_header("Accept-Encoding".to_owned(), "br, zstd".to_owned());
        resp.set_body(body.to_vec(), &req).expect("I/O Error");

        assert_eq!(resp.body, body);
    }

    #[test]
    fn safe_path() {
        let safe_path = "images/more_images/banana.jpg";

        assert!(is_path_safe(safe_path));
    }

    #[test]
    fn unsafe_path_parent_travesal() {
        let unsafe_path = "../images";

        assert!(!is_path_safe(unsafe_path));
    }
}