use std::path::PathBuf;

use clap::Parser;
use transacty::{
    process_events,
    state::{memory::MemoryState, StateManager},
};

#[derive(Parser, Debug)]
struct Cli {
    /// Path to the input CSV file.
    #[clap(parse(from_os_str))]
    input: PathBuf,

    /// Emit errors to stdout during processing.
    #[clap(short, long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(&cli.input)?;

    let mut errors = None;
    let join_handle = if cli.debug {
        let (tx, rx) = std::sync::mpsc::sync_channel(16);
        errors = Some(tx);
        Some(std::thread::spawn(move || {
            use std::io::Write;

            let stderr = std::io::stderr();
            let mut stderr = stderr.lock();

            while let Ok(err) = rx.recv() {
                writeln!(stderr, "{err}").expect("writing to stderr never panics");
            }
        }))
    } else {
        None
    };

    let mut state = MemoryState::default();
    process_events(
        &mut state,
        reader
            .into_deserialize()
            .map(|maybe_event| maybe_event.expect("csv files are valid throughout")),
        errors,
    );

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = csv::Writer::from_writer(stdout);

    for client_state in state.emit_state() {
        writer.serialize(client_state)?;
    }

    // wait for all errors to be emitted before exiting
    if let Some(handle) = join_handle {
        handle.join().expect("this thread never panics");
    }

    Ok(())
}
