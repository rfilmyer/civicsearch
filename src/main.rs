use std::process;
use std::fs;
use std::io::Cursor;
use shapefile::Polygon;
use zip::{ZipArchive};
use log::{error};

fn main() {
    env_logger::init();
    const DEFAULT_ZIPFILE_PATH: &str = "tl_2019_25_sldl.zip";

    let zipfile_buffer = fs::read(DEFAULT_ZIPFILE_PATH)
        .unwrap_or_else(|err| {error!("Problem reading zip file: {:?}", err); process::exit(1)});
    let zipfile_buffer = Cursor::new(zipfile_buffer);

    let zipfile_archive = ZipArchive::new(zipfile_buffer)
        .unwrap_or_else(|err| {error!("Problem reading zip file: {:?}", err); process::exit(1)});
    let reader = civicsearch::shapefile_reader_from_zip_archive(zipfile_archive)
        .unwrap_or_else(|err| {error!("Problem parsing shapefile: {:?}", err); process::exit(1)});

    const LAT: f64 = 42.389408;
    const LON: f64 = -71.119699;

    let point: geo_types::Point<f64> = geo_types::Point::new(LON, LAT);

    let matching_districts = reader
        .iter_shapes_and_records_as::<Polygon>()
        .unwrap_or_else(|err| {eprintln!("Problem reading shapefile: {}", err); process::exit(1)})
        .filter_map(|sr| sr.ok())
        .filter(|(s, _)| civicsearch::shape_contains_point(s, &point))
        .map(|(_, r)| civicsearch::extract_district_name(r))
        .collect::<Option<Vec<String>>>()
        ;
    
    match matching_districts {
        Some(d) => {
            println!("Matching districts:");
            for district in d {
                println!("{}", district)
            }
        },
        None => {
            println!("No Matching Districts Found.");
        }
    }

}