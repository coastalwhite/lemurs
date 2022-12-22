use lemurs::auth::{AuthError, SessionUser};
use lemurs::session_environment::SessionEnvironment;
use std::io::{self, BufRead, stdout, Write};

fn main() -> io::Result<()> {
    let mut username = String::new();
    let mut password = String::new();

    loop {
        let session_user = loop {
            let mut stdin = io::stdin().lock();

            println!("Please login");

            // Read login information
            print!("username: ");
            stdout().lock().flush()?;
            stdin.read_line(&mut username)?;

            print!("password: ");
            stdout().lock().flush()?;
            stdin.read_line(&mut password)?;

            // Try to authenticate the user.
            match SessionUser::authenticate(&username, &password) {
                Ok(session_user) => break session_user,
                Err(_) => eprintln!("Authentication failure!"),
            }
        };

        // Start a shell with the the given session
        match SessionEnvironment::Shell.start(&session_user) {
            Ok(_) => println!("Welcome back!"),
            Err(_) => {
                eprintln!("Failed to start TTY");
                std::process::exit(1);
            }
        }
    }
}
