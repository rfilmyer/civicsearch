# civicsearch
A tool to get state legislative districts from latitude/longitude points on a map.

## Current State
* Under active developent, but as of 0.2.0 this tool should be usable.
* `civicsearch` is now a command line tool.
* Downloads available at the [Releases](https://github.com/rfilmyer/civicsearch/releases/) page.

### How to use
Run on the command line.

* Get help with `./civicsearch --help` (or `./civicsearch.exe --help` on Windows)
* Supply a CSV file with 2 columns - `latitude` and `longitude` with `--input`
* Supply a ZIP file from [TIGER](https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html) with `--shapefile`.

Example:
`civicsearch.exe --input example_locations.csv --shapefile .\tl_2019_25_sldl.zip`

## Future

Although `civicsearch` currently works on the command line, I believe I can repurpose this tool to work on a static webpage using webassembly. Stay tuned! Should be much more user-friendly when I get that done.
