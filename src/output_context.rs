use crate::error::Result;
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::sync::{Mutex, OnceLock};

enum OutputDestination {
    Stdout,
    File(File),
    Sink,
}

struct OutputContext {
    destination: OutputDestination,
}

static OUTPUT_CONTEXT: OnceLock<Mutex<OutputContext>> = OnceLock::new();

pub fn configure(benchmark: bool, output_path: Option<&str>) -> Result<()> {
    let destination = match output_path {
        Some(path) => OutputDestination::File(File::create(path)?),
        None if benchmark => OutputDestination::Sink,
        None => OutputDestination::Stdout,
    };

    let context = OUTPUT_CONTEXT.get_or_init(|| {
        Mutex::new(OutputContext {
            destination: OutputDestination::Stdout,
        })
    });

    if let Ok(mut context) = context.lock() {
        context.destination = destination;
    }

    Ok(())
}

pub fn write_line(line: &str) {
    let context = OUTPUT_CONTEXT.get_or_init(|| {
        Mutex::new(OutputContext {
            destination: OutputDestination::Stdout,
        })
    });

    let Ok(mut context) = context.lock() else {
        return;
    };

    match &mut context.destination {
        OutputDestination::Stdout => {
            println!("{}", line);
        }
        OutputDestination::File(file) => {
            let _ = writeln!(file, "{}", line);
        }
        OutputDestination::Sink => {}
    }
}

pub fn should_use_color() -> bool {
    let context = OUTPUT_CONTEXT.get_or_init(|| {
        Mutex::new(OutputContext {
            destination: OutputDestination::Stdout,
        })
    });

    let Ok(context) = context.lock() else {
        return false;
    };

    matches!(context.destination, OutputDestination::Stdout) && io::stdout().is_terminal()
}

pub fn is_stdout() -> bool {
    let context = OUTPUT_CONTEXT.get_or_init(|| {
        Mutex::new(OutputContext {
            destination: OutputDestination::Stdout,
        })
    });

    let Ok(context) = context.lock() else {
        return false;
    };

    matches!(context.destination, OutputDestination::Stdout)
}
