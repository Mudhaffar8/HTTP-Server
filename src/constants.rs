// TCP Connection
pub const ADDRESS: &'static str = concat!("127.0.0.1", ":", "2109");
pub const NUM_OF_THREADS: usize = 6;

// Compression
pub const VALID_COMPRESSION_MODES: [&'static str; 1] = ["gzip"];

// Web URLs
pub const PATH_TO_HOMEPAGE: &'static str = "./example-website/index.html";
pub const PATH_TO_CSS: &'static str = "./example-website/css/";
pub const PATH_TO_IMAGES: &'static str = "./example-website/images/";