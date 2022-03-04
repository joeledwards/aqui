use std::io;

fn inputPrompt() -> Option<String> {
    print!("Please input a value: ");
    let mut input = String::new();
    let result = io::stdin().read_line(&mut input);
    match result {
        Ok(value) => Some(value),
        Err(_) => None
    }
}