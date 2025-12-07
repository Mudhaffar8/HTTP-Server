# HTTP Web Server

<img width="1585" height="1032" alt="image" src="https://github.com/user-attachments/assets/6cc0c4df-0d84-4888-b307-803f7b3bdd34" />

This is a lightweight HTTP web server built in Rust. It currently supports:
- Route request handling for HTML, CSS, images, etc.
- Thread pooling for handling multiple connections
- Gzip compression support via the flate2 crate (with plans to add Zlib and Deflate)
- Path safety checks to prevent directory-traversal attacks
