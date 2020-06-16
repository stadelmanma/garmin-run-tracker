use garmin_run_tracker::{create_database, import_fit_data};
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

/// Parse FIT formatted files and import their data into the local database
#[derive(Debug, StructOpt)]
#[structopt(name = "fit_to_json")]
struct Cli {
    /// FIT files to import
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::from_args();

    // create database if needed
    create_database()?;

    // Read each FIT file and import it
    for file in opt.files {
        // open file and parse data
        let mut fp = File::open(&file)?;
        let data = fitparser::from_reader(&mut fp)?;
        import_fit_data(&data)?;
        println!("Successfully imported FIT file: {:?}", file);
    }

    Ok(())
}
