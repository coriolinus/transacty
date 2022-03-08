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
    // /// Emit errors to stdout during processing.
    // #[clap(short, long)]
    // debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let reader = csv::Reader::from_path(&cli.input)?;

    let mut state = MemoryState::default();
    process_events(
        &mut state,
        reader
            .into_deserialize()
            // TODO: this assumes no CSV errors in input files; accurate?
            .filter_map(|maybe_event| maybe_event.ok()),
        None,
    );

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = csv::Writer::from_writer(stdout);

    for client_state in state.emit_state() {
        writer.serialize(client_state)?;
    }

    Ok(())
}
