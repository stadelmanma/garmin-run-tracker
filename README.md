# Garmin Run Tracker
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Overview

A basic command line application to parse FIT files from a Garmin watch
and store them in a local sqlite3 database. The database file by default
is stored at `$XDG_DATA_HOME/garmin-run-tracker.db` on linux systems. The
[dirs::data_dir](https://docs.rs/dirs/2.0.2/dirs/fn.data_dir.html) function
is used to provide the path to the user's data directory. Please refer to
its documentation to determine the default path on other operating systems.

Once imported data can be easily viewed and manipulated via the sqlite
command line interface or a program that connects to the database. The
schema is simple and can be viewed in `src/schema.rs`.

```sql
-- Connect to the DB using: sqlite3 ~/.local/share/garmin-run-tracker.db
-- View some import files
select * from files limit 5;
/*
type|manufacturer|product|time_created|serial_number|id
activity|garmin|fr25|2017-12-29T20:59:24+00:00|3956226596|1
activity|garmin|fr25|2018-05-01T21:41:24+00:00|3956226596|2
activity|garmin|fr25|2018-05-07T19:55:01+00:00|3956226596|3
activity|garmin|fr25|2018-05-08T21:41:58+00:00|3956226596|4
activity|garmin|fr25|2018-05-10T21:44:14+00:00|3956226596|5
*/

-- Select your top 5 fastest miles
select average_speed, average_heart_rate, total_distance, timestamp
    from lap_messages
    where total_distance > 1600 and average_heart_rate is not null
    order by average_speed desc limit 5;
/*
average_speed|average_heart_rate|total_distance|timestamp
3.633|167|1609.34|2018-03-02T21:39:36+00:00
3.609|172|1609.34|2018-02-21T21:32:30+00:00
3.558|135|1609.34|2018-06-22T20:49:23+00:00
3.553|164|1609.34|2018-05-17T22:30:33+00:00
3.553|164|1609.34|2018-05-17T22:30:33+00:00
*/
```

## Features

### Duplicate File Detection

Duplicate files are currently detected by taking the SHA256 has of the
entire content and then truncating it down into a 128bit UUID format for
storage. This method is robust in that if even a single byte changes it
is a "new" file. However, it is also very IO intensive since the duplicate
file still gets read in it's entirety. This process could be easily
reimplemented in parallel to speed up the wall clock time for large import
sets. A second less robust, but much faster, method would be just checking
filenames if the import location is known to use a unique naming convention.

### Adding Elevation Data

Elevation data does not always comes with the watch but generally can be
obtained via various APIs from third-party sources. At the moment if
elevation data is desired one will need to setup a locally hosted instance
of [opentopodata](https://www.opentopodata.org/). Implementing
an interface that would allow other providers to be used is something
that will be considered at a later date.

See their documentation on how to setup a local instance and load it with
your desired elevation dataset (if needed). The
[ned](https://www.opentopodata.org/datasets/ned/) dataset is used here
which maps nearly the all of North America at 30m and US in higher
10m resolution.

The config.yml used to setup the local server.
```yaml
# 400 error will be thrown above this limit.
max_locations_per_request: 100

# CORS header. Should be None for no CORS, '*' for all domains, or a url with
# protocol, domain, and port ('https://api.example.com/'). Default is null.
access_control_allow_origin: null

# config for 1/3 arcsecond NED data
datasets:
  - name: ned10m
    path: data/ned10m/
    filename_epsg: 4326  # This is the default value.
    filename_tile_size: 1  # This is the default value.
```


### Future

Additional features are being considered/planned out, such as:
 * Output various statistics like "Personal Bests"
 * Output other aggregate data like weekly mileage
 * Create static route map images using the lat/long positions
 * Add data plotting capabilities
 * Implementing the elevation logic as a trait so end users can
   more easily use their own source(s).
 * ...
