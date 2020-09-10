# civicsearch
A tool to get state legislative districts from latitude/longitude points on a map.

## Current State

* It works?
* A command line tool that determines the legislative district for a hardcoded point.

## Vision
This will be an interactive tool. It should:

* Have a more user-friendly frontend; either wasm, electron, or a native app.
* Ingest data via a specially-formatted csv, or excel spreadsheet.
* Ingest shapefiles from [TIGER](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html), either as a `.zip` or as the three important files contained in the `.zip` - the `.shp` (holding shapes), the `.dbf` (holding metadata like district names), and the `.shx` (an index file connecting the other two).