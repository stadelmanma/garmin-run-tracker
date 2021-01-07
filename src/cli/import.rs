//! Define FIT file import command
use crate::config::Config;
use crate::services::{update_elevation_data, ElevationDataSource};
use crate::{devices_dir, import_fit_data, open_db_connection, Error, FileInfo};
use log::{debug, error, info, trace, warn};
use rusqlite::Connection;
use std::fs::{copy as copy_file, create_dir_all, read_dir, File};
use std::path::PathBuf;
use structopt::StructOpt;

/// Import one or more FIT files directly or within the provided directories
#[derive(Debug, StructOpt)]
pub struct ImportOpts {
    /// FIT files to import or directories to search
    #[structopt(name = "PATHS", parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// Search directory paths recursively
    #[structopt(short, long)]
    recursive: bool,
    /// Do not copy imported FIT files into the devices directory
    #[structopt(long)]
    no_copy: bool,
    /// Do not search the import paths defined in the application config
    #[structopt(long)]
    skip_config_paths: bool,
    /// Attempt to pull elevation data for rows in the database that are currently NULL
    #[structopt(long)]
    fix_missing_elevation: bool,
    /// Do not query elevation service when importing data
    #[structopt(long)]
    no_elevation: bool,
}

/// Implementation of the `import` subcommand
pub fn import_command(config: Config, opts: ImportOpts) -> Result<(), Box<dyn std::error::Error>> {
    // fetch elecation service from config
    let elevation_hdl = if !opts.no_elevation {
        match config.get_elevation_handler() {
            Ok(hdl) => Some(hdl),
            Err(e) => {
                if opts.fix_missing_elevation {
                    return Err(Box::new(e)); // hard error if we specifically wanted elevation import
                }
                error!("Could not initialize the elevation service {}", e);
                None
            }
        }
    } else {
        None
    };

    // merge paths from config with any user provided ones
    let mut import_paths: Vec<PathBuf> = if opts.skip_config_paths {
        Vec::new()
    } else {
        config
            .import_paths()
            .iter()
            .map(|s| PathBuf::from(s))
            .collect()
    };
    import_paths.extend(opts.paths);

    // throw an error for no import paths unless we are fixing elevation
    if import_paths.len() == 0 && !opts.fix_missing_elevation {
        return Err(Box::new(Error::Other(
            "No import paths provided".to_string(),
        )));
    }

    // Import FIT files from the defined paths
    let mut conn = open_db_connection()?;
    import_files(
        &mut conn,
        &import_paths,
        opts.recursive,
        import_paths.len() == 1, // allow hard error on dupe if our only path is a single file
        !opts.no_copy,
        elevation_hdl.as_ref(),
    )?;

    // update missing elevation data in database, we'll hard error here if this fails since
    // the task was requested directly, overwrite = false to only hit missed values.
    if opts.fix_missing_elevation {
        if let Some(hdl) = elevation_hdl {
            update_elevation_data(&hdl, None, false)?;
        } else {
            return Err(Box::new(Error::Other(
                "Could not fix missing elevation data, no elevation service available".to_string(),
            )));
        }
    }

    Ok(())
}

/// import multiple files into the database as well as handle recursive directory searches
fn import_files(
    conn: &mut Connection,
    paths: &[PathBuf],
    recursive: bool,
    err_on_dupe: bool,
    persist_file: bool,
    elevation_hdl: Option<&impl ElevationDataSource>,
) -> Result<(), Error> {
    for path in paths {
        if !path.exists() {
            warn!("Path does not exist: {:?}", path);
            continue;
        }
        if path.is_dir() {
            debug!("Scanning contents of: {:?} for FIT files", path);
            // collect files with the "FIT" extension from the directory and if we are processing
            // directories recursively incldue them in the import call.
            let new_paths = read_dir(path)?;
            let new_paths: Vec<PathBuf> = new_paths
                .filter_map(|d| d.ok())
                .map(|d| d.path())
                .filter(|p| {
                    p.is_dir() && recursive
                        || p.extension()
                            .map_or(false, |e| e.to_string_lossy().to_ascii_lowercase() == "fit")
                })
                .collect();
            // call function with found paths, `err_on_dupe` is set to false since we're recursing
            import_files(
                conn,
                &new_paths,
                recursive,
                false,
                persist_file,
                elevation_hdl,
            )?;
        } else {
            match import_file(conn, path, persist_file, elevation_hdl) {
                Ok(_) => {}
                Err(e) => {
                    // handle various errors
                    match &e {
                        Error::DuplicateFileError(_) => {
                            if err_on_dupe {
                                // if we are importing a single file and it's a dupe throw a hard error
                                error!("{}", e);
                                return Err(e);
                            } else {
                                // if we are importing multiple files or are being call recursively,
                                // just warn about the dupe instead
                                warn!("{}", e);
                                continue;
                            }
                        }
                        _ => return Err(e), // propagate all other errors
                    }
                }
            }
        }
    }

    Ok(())
}

/// Import a FIT files into the database, optionally fetching elevation data from an external service
fn import_file(
    conn: &mut Connection,
    file: &PathBuf,
    persist_file: bool,
    elevation_hdl: Option<&impl ElevationDataSource>,
) -> Result<FileInfo, Error> {
    trace!("Importing FIT file: {:?}", file);
    let tx = conn.transaction()?;
    let mut fp = File::open(&file)?;
    let file_info = import_fit_data(&mut fp, &tx)?;
    info!(
        "Successfully imported FIT file: {:?} (UUID={})",
        &file,
        file_info.uuid()
    );
    tx.commit()?;

    // copy FIT file to a local storage location since the device itself will delete the
    // file when it needs space.
    if persist_file {
        let sub_dir_name = format!(
            "{}-{}-{}",
            file_info.manufacturer(),
            file_info.product(),
            file_info.serial_number()
        );
        let mut dest = devices_dir().join(&sub_dir_name);
        if !dest.exists() {
            create_dir_all(&dest)?;
        }
        match file.file_name() {
            Some(name) => dest.push(name),
            None => dest.push(&format!("{}.fit", file_info.uuid())),
        };
        copy_file(&file, &dest)?;
        info!("Successfully copied FIT file {:?} to {:?}", &file, &dest);
    }

    // add elevation data if possible, we overwrite here on the assumption that API provides
    // more accurate values than the device.
    if let Some(hdl) = elevation_hdl {
        match update_elevation_data(hdl, Some(file_info.uuid()), true) {
            Ok(_) => {
                info!(
                    "Successfully imported elevation for FIT file '{}'",
                    file_info.uuid()
                );
            }
            Err(e) => {
                error!(
                    "Could not import elevation data from the API for FIT file '{}'",
                    file_info.uuid()
                );
                error!("{}", e)
            }
        }
    }

    Ok(file_info)
}
