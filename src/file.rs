/// Read a test file and store it into a buffer.
pub fn read_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            println!("Could not open file: {}", error);
            std::process::exit(1);
        }
    }
}
