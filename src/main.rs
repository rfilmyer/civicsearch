use std::process;
use std::fs;
use std::io::Cursor;
use shapefile::Polygon;
use zip::{ZipArchive};
use log::{error, info};
use clap::{Arg, App};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::ffi::OsStr;

#[derive(Debug, Deserialize, Copy, Clone)]
struct CSVInputRecord {
    latitude: f64,
    longitude: f64
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CSVOutputRecord<'a> {
    latitude: f64,
    longitude: f64,
    district: Option<&'a str>,
    other_districts: Option<&'a str>,
}
fn main() {
    env_logger::init();

    // cli running
    let matches = App::new("CivicSearch")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("input")
            .short("i")
            .long("input")
            .value_name("EXAMPLE_CSV")
            .help("The path to your latitude/longitude coordinate dataset. Should be a CSV with 2 columns, 'latitude' and 'longitude'")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("shapefile")
            .short("s")
            .long("shapefile")
            .value_name("SHAPEFILE_ZIP")
            .help("The location of a TIGER shapefile zip file. Please use the whole .zip file and not a .shp")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("OUTPUT_CSV")
            .value_name("The path for the output CSV (with latitude, longitude, and district columns). out.csv by default.")
            .takes_value(true)
        )
        .get_matches();
    
    // parse path args and convert them into paths
    let zipfile_path = matches.value_of_os("shapefile")
        .unwrap_or_else(|| {println!("Could not find path of shapefile."); process::exit(1)});
    let zipfile_path = Path::new(zipfile_path);

    let csv_path = matches.value_of_os("input")
        .unwrap_or_else(|| {println!("Could not find path of CSV file."); process::exit(1)});
    let csv_path = Path::new(csv_path);

    // Open coordinates csv file
    let mut points: Vec<geo_types::Point<f64>> = Vec::new();
    let mut rdr = csv::Reader::from_path(csv_path)
        .unwrap_or_else(|err| {error!("Problem loading csv file: {:?}", err); process::exit(1)});
    for result in rdr.deserialize() {
        let record: CSVInputRecord = result
            .unwrap_or_else(|err| {error!("Problem reading line in CSV: {:?}", err); process::exit(1)});
        let point = geo_types::point!(x: record.longitude, y: record.latitude);
        points.push(point);
    }
    info!("Found {} points in {}", points.len(), csv_path.display());

    // Open shapefile zip file
    let zipfile_buffer = fs::read(zipfile_path)
        .unwrap_or_else(|err| {error!("Problem loading zip file: {:?}", err); process::exit(1)});
    let zipfile_buffer = Cursor::new(zipfile_buffer);

    let zipfile_archive = ZipArchive::new(zipfile_buffer)
        .unwrap_or_else(|err| {error!("Problem reading zip file: {:?}", err); process::exit(1)});
    let reader = civicsearch::shapefile_reader_from_zip_archive(zipfile_archive)
        .unwrap_or_else(|err| {error!("Problem parsing shapefile: {:?}", err); process::exit(1)});

    let district_map: Vec<(Polygon, shapefile::dbase::Record)> = reader
        .iter_shapes_and_records_as::<Polygon>()
        .unwrap_or_else(|err| { error!("Could not read shapefile data: {:?}", err); process::exit(1)})
        .flat_map(|sr| sr.ok())
        .collect();
    
    info!("Loaded district map with {} districts from {}", district_map.len(), zipfile_path.display());
    
    let points_with_districts = civicsearch::find_districts_for_points(points.iter(), district_map.iter());


    // Write to CSV
    let output_path = matches.value_of_os("output")
        .unwrap_or(OsStr::new("out.csv"));

    let output_path = Path::new(output_path);
    let mut wtr = csv::Writer::from_path(output_path)
        .unwrap_or_else(|err| {error!("Problem opening output file: {:?}", err); process::exit(1)});
    
    
    for (point, districts) in &points_with_districts {
        let (first_district, other_districts): (Option<&String>, Option<&[String]>) = match districts.split_first() {
            Some((f, o)) if !o.is_empty() => (Some(f), Some(o)),
            Some((f, _))                  => (Some(f), None),
            None => (None, None),
        };

        let other_districts =  match other_districts {
            Some(o) => Some(o.join(",")),
            None => None
        };
        let record = CSVOutputRecord {
            latitude: point.y(), 
            longitude: point.x(), 
            district: first_district.map(String::as_ref),
            other_districts: other_districts.as_deref(),
        };
        wtr.serialize(record)
            .unwrap_or_else(|err| {error!("Problem writing record {:?} to file: {:?}", record, err);});
    }

    wtr.flush()
        .unwrap_or_else(|err| { println!("Problem flushing CSV write buffer (this is VERY weird): {:?}", err) });
}
