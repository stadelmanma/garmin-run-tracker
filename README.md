# Garmin Run Tracker
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Overview

A basic command line application to parse FIT files from a Garmin watch
and store them in a local sqlite3 database. The database file by default
is stored at `$XDG_DATA_HOME/garmin-run-tracker/garmin-run-tracker.db`
on linux systems. The
[dirs::data_dir](https://docs.rs/dirs/2.0.2/dirs/fn.data_dir.html)
function is used to provide the path to the user's data directory.
Please refer to its documentation to determine the default path on other
operating systems.

See `garmin_run_tracker --help` for usage information on the command line
interface.

Once imported data can be easily viewed and manipulated via the sqlite
command line interface or a program that connects to the database. The
schema is simple and can be viewed in `src/db/schema.rs` or via the
`.schema` command in the SQLite console.


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
    where total_distance > 1600
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

## Configuration
Configuration of the program is done through a YAML file located at
`$XDG_DATA_HOME/garmin-run-tracker/config.yml`. An example file is
located at the root of this project (config-example.yml) and can be
copied into that location as a starting point. The configuration file
defines a default log level, automatic import paths and sets parameters
for external services used by the application.

## Features


### Duplicate File Detection

Duplicate files are currently detected by taking the SHA256 hash of the
entire content and then truncating it down into a 128bit UUID format for
storage. This method is robust in that if even a single byte changes it
is a "new" file. However, it is also very IO intensive since the duplicate
file still gets read in it's entirety. This process could be easily
reimplemented in parallel to speed up the wall clock time for large import
sets. A second less robust, but much faster, method would be just checking
filenames if the import location is known to use a unique naming convention.


### Adding Elevation Data

Elevation data does not always comes with the watch but generally can be
obtained via various APIs from third-party sources. This code was developed
using a locally hosted instance of [opentopodata](https://www.opentopodata.org/).
However, any data source can be configured to work via the `ElevationDataSource`
trait which requires a single method to be implemented. That method
`request_elevation_data` fetches elevation data for a vector of latitude
and longitude coordinate pairs, stored as a `Location` struct.

See their documentation on how to setup a local instance and load it with
your desired elevation dataset (if needed). The
[ned](https://www.opentopodata.org/datasets/ned/) dataset is used here
which maps nearly the all of North America at 30m and US in higher
10m resolution.

The config.yml used to setup the local opentopodata server. NOTE: This
is not the same config used by the garmin-run-tracker application.
(See the config-example.yaml file for how we configure our access to this
service).
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

It would be trivial to swap out the locally hosted version of opentopodata
for the public API hosted at https://api.opentopodata.org/ but one would
need to be wary of rate limiting. The current code implementation makes
no effort to "delay" requests to obey API guidelines since it's not
necessary for a locally hosted version.


### Static Route Images

Static route images are generated using third party services that provide
map tiles and/or route plotting capabilities. This code was developed using
a locally hosted instance of [openmaptiles](https://openmaptiles.org/) as
well as the [MapBox API](https://www.mapbox.com/).

Support for this feature is done through the `RouteDrawingService` trait
which has a `draw_route` method. The route drawing service accepts a GPS
trace (of the form `&[Location]`) and a slice of `&[Marker]` structs
that can be used to define mile markers, start and end points, etc.
(if supported).

#### Default Configurations for Route Drawers

Below is the deafault configuration options for each service. Only a single
handler can be defined right now and not all features of the external
service may be supported.

##### MapBox
See API docs here: https://docs.mapbox.com/api/maps/static-images/
```yaml
services:
    route_visualization:
        handler: mapbox
        configuration:
            base_url: "https://api.mapbox.com"
            api_version: "v1"
            username: "mapbox"
            style: "streets-v11"  # map style, several are offered
            image_width: 1280  # These are the maximum image dimensions
            image_height: 1280
            marker_color: "f07272"  # any hexcode color for mile markers
            marker_style: "l"  # Can be "l" (large) or "s" (small)
            stroke_color: "f44"  # any hexcode color for the GPS trace line
            stroke_width: 5
            stroke_opacity: 0.75
            access_token: null  # required API access token
```


##### OpenMapTiles
See API docs here: https://support.maptiler.com/i26-static-maps-for-your-web
```yaml
services:
    route_visualization:
        handler: openmaptiles
        configuration:
            base_url: http://localhost:8080  # locally hosted by default
            style: osm-bright  # map tile style
            image_width: 1800
            image_height: 1200
            image_format: png  # PNG image format (jpg also supported)
            stroke_color: red  # Color of the GPS trace line
            stroke_width: 3
```


### EPO Data Downloading

EPO data can be downloaded from the Garmin website and stored on your watch.
This was tested with a Forerunner 25 and uses the same logic as the
[postrunner](https://github.com/scrapper/postrunner) application. The
`epo_data_paths` top level key can specify one or more locations to save
EPO data for.

```yaml
# locations to save download EPO data to (usually this will be
# /[mount-point]/GARMIN/GARMIN/REMOTESW/EPO.BIN)
epo_data_paths:
    - /media/mstadelman/GARMIN/GARMIN/REMOTESW/EPO.BIN
```


### Data Plotting

A simple terminal-based plotting handler is provided and can be used via
the `show` sub command. This will plot the pace, elevation and heart rate
as a function of distance. The terminal based plotting is simplistic but
allows for quick visualization of key data. As of right now it does not
accept any changes based on configuration and serves as the default and
only data plotting service when one isn't defined.


### Future

Additional features are being considered/planned out, such as:
 * Output various statistics like "Personal Bests"
 * Output other aggregate data like weekly mileage
 * Allow runs to be labeled/named, i.e. "morgantown marathon"
 * Allow comments on runs
 * ...
