use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use infer;


fn handle_client(mut stream: TcpStream, base_dir: &PathBuf) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    
    let request = String::from_utf8_lossy(&buffer[..]);
    let requested_path = parse_request(&request);
    
    // Sanitize and resolve path
    let path = resolve_path(base_dir, &requested_path);
    if path.as_ref().map_or(true, |p| !p.starts_with(base_dir)) {
        send_404(&mut stream);
        return;
    }
    
    let path = path.unwrap();
    
    if path.is_dir() {
        // Serve directory listing
        let response = generate_directory_listing(&path);
        send_response(&mut stream, &response, "text/html");
    } else if path.is_file() {
        // Serve file
        let content_type = infer::get_from_path(&path).unwrap_or(None)
            .map(|kind| kind.mime_type())
            .unwrap_or("application/octet-stream");
        send_file(&mut stream, &path, content_type);
    } else {
        send_404(&mut stream);
    }
}

fn parse_request(request: &str) -> String {
    let mut lines = request.lines();
    if let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let requested_path = parts[1].to_string();
            if requested_path == "/" {
                return ".".to_string(); // Return current directory if root is requested
            }
            return requested_path;
        }
    }
    "/".to_string()
}

fn resolve_path(base_dir: &PathBuf, requested_path: &str) -> Option<PathBuf> {
    // Fix here to use into_owned() since Cow<'_, str> is returned
    let decoded_path = url_escape::decode(requested_path).into_owned();
    let mut full_path = base_dir.clone();
    full_path.push(Path::new(&decoded_path));

    full_path.canonicalize().ok()
}





fn generate_directory_listing(path: &Path) -> String {
    let mut html = String::new();
    html.push_str("<html><head><title>Directory Listing</title></head><body>");
    html.push_str("<h1>Directory Listing</h1><ul>");

    for entry in walkdir::WalkDir::new(path).max_depth(1) {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy();
        let file_path = entry.path().display().to_string();

        // Make the file path URL-friendly (handle special characters)
        let encoded_path = url_escape::encode_component(&file_path).to_string();

        // Create a clickable link
        html.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            encoded_path, file_name
        ));
    }

    html.push_str("</ul></body></html>");
    html
}


fn send_response(stream: &mut TcpStream, content: &str, content_type: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        content_type,
        content.len(),
        content
    );
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn send_file(stream: &mut TcpStream, path: &Path, content_type: &str) {
    let file_content = fs::read(path).unwrap();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        content_type,
        file_content.len()
    );
    stream.write(response.as_bytes()).unwrap();
    stream.write(&file_content).unwrap();
    stream.flush().unwrap();
}

fn send_404(stream: &mut TcpStream) {
    let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main() {
    // Start the TCP listener on port 7878
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    
    // Print a message to confirm the server is running
    println!("Server is listening on http://127.0.0.1:7878");

    let base_dir = std::env::current_dir().unwrap();

    // Accept incoming connections
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_client(stream, &base_dir);
    }
}

