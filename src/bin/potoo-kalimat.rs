use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use potoo_kalimat_interpreter::{Host, Interpreter, parse};

struct CliHost {
    random_state: u64,
    unit_root: PathBuf,
}

impl Host for CliHost {
    fn random(&mut self, bound: i64) -> Result<i64, String> {
        if bound <= 0 {
            return Err("random bound must be positive".into());
        }
        self.random_state ^= self.random_state << 13;
        self.random_state ^= self.random_state >> 7;
        self.random_state ^= self.random_state << 17;
        Ok((self.random_state % bound as u64) as i64)
    }

    fn draw_line(&mut self, _from: (i64, i64), _to: (i64, i64), _color: i64) -> Result<(), String> {
        Err("drawLine requires a graphical host".into())
    }

    fn wait(&mut self, milliseconds: i64) -> Result<(), String> {
        let milliseconds = u64::try_from(milliseconds)
            .map_err(|_| "wait duration cannot be negative".to_owned())?;
        std::thread::sleep(Duration::from_millis(milliseconds));
        Ok(())
    }

    fn load_unit(&mut self, file_name: &str) -> Result<Option<String>, String> {
        let path = self.unit_root.join(file_name);
        match std::fs::read_to_string(&path) {
            Ok(source) => Ok(Some(source)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: potoo-kalimat <source-file>".to_owned())?;
    let source = std::fs::read_to_string(&path).map_err(|error| format!("{path}: {error}"))?;
    let unit_root = Path::new(&path)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let program = parse(&source).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos() as u64
        | 1;
    Interpreter::new(CliHost {
        random_state: seed,
        unit_root,
    })
    .run(&program)
    .map_err(|error| error.to_string())?;
    Ok(())
}
