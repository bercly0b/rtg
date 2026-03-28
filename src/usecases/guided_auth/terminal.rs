use std::io;

use super::AuthTerminal;

pub struct StdTerminal;

impl AuthTerminal for StdTerminal {
    fn print_line(&mut self, line: &str) -> io::Result<()> {
        println!("{line}");
        Ok(())
    }

    fn prompt_line(&mut self, prompt: &str) -> io::Result<Option<String>> {
        use std::io::Write;

        print!("{prompt}");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes = io::stdin().read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }

        Ok(Some(line.trim().to_owned()))
    }

    fn prompt_secret(&mut self, prompt: &str) -> io::Result<Option<String>> {
        match rpassword::prompt_password(prompt) {
            Ok(password) => Ok(Some(password)),
            Err(source) if source.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(source) => Err(source),
        }
    }
}
