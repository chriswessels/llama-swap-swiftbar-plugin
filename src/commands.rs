pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "test" => {
            println!("Command handling works!");
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}