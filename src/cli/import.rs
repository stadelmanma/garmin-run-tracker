//! Define FIT file import command
use crate::config::Config;
use crate::services::{update_elevation_data, ElevationDataSource};
use crate::{devices_dir, import_fit_data, Error, FileInfo};
use log::{error, info, trace, warn};
use std::fs::{copy as copy_file, create_dir_all, read_dir, File};
use std::path::PathBuf;
use structopt::StructOpt;

/// Generate an image of the running route based on the file's waypoints
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

    // Import each fit file
    for path in import_paths {
        if path.is_dir() {
            // TODO: process directory, recursively if need be
        } else {
            import_file(path, opts.no_copy, elevation_hdl.as_ref())?;
            //TODO handle the dupe file error:
            // Err(e) => {
            //     // handle various errors
            //     match &e {
            //         Error::DuplicateFileError(_) => {
            //             if opts.files().len() == 1 {
            //                 // if we are importing a single file and it's a dupe throw a hard error
            //                 error!("{}", e);
            //                 return Err(Box::new(e));
            //             } else {
            //                 // if we are importing multiple files, just warn about the dupe
            //                 warn!("{}", e);
            //                 return;
            //             }
            //         }
            //         _ => return Err(Box::new(e)),
            //     }
            // }
        }
    }

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

fn import_file(
    file: PathBuf,
    no_copy: bool,
    elevation_hdl: Option<&impl ElevationDataSource>,
) -> Result<FileInfo, Error> {
    trace!("Importing FIT file: {:?}", file);
    let mut fp = File::open(&file)?;
    let file_info = import_fit_data(&mut fp)?;

    // copy FIT file to a local storage location since the device itself will delete the
    // file when it needs space.
    if !no_copy {
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

    info!(
        "Successfully imported FIT file: {:?} (UUID={})",
        &file,
        file_info.uuid()
    );
    // add elevation data if possible, we overwrite here on the assumption that API is
    // more accurate value than the device.
    info!(
        "Attempting to update elevation data for FIT file: {:?} (UUID={})...",
        &file,
        file_info.uuid()
    );

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
