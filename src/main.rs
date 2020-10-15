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
        .unwrap_or_else(|err| {error!("Problem loading zip file: {:?}", err); process::exit(1)});
    let zipfile_buffer = Cursor::new(zipfile_buffer);

    let zipfile_archive = ZipArchive::new(zipfile_buffer)
        .unwrap_or_else(|err| {error!("Problem reading zip file: {:?}", err); process::exit(1)});
    let reader = civicsearch::shapefile_reader_from_zip_archive(zipfile_archive)
        .unwrap_or_else(|err| {error!("Problem parsing shapefile: {:?}", err); process::exit(1)});

    let points = vec!(
        geo_types::point!(x: -71.1196990, y: 42.3894080), // Yume wo Katare, 25th Middlesex
        geo_types::point!(x: -71.0596335, y: 42.3744386), // Bunker Hill Monument, 2nd Suffolk
        geo_types::point!(x: -71.0374142, y: 42.3669590), // KO Meat Pies, 1st Suffolk
    );

    let district_map: Vec<civicsearch::ShapeWithRecord> = reader
        .iter_shapes_and_records_as::<Polygon>()
        .unwrap_or_else(|err| { error!("Could not read shapefile data: {:?}", err); process::exit(1)})
        .flat_map(|sr| sr.ok())
        .map(|(shape, record)| civicsearch::ShapeWithRecord { shape, record })
        .collect();
    let points_with_districts = civicsearch::find_districts_for_points(points.iter(), district_map.iter());

    for (point, districts) in points_with_districts {
        println!("Point: {}, {} - Matching districts: {}", point.x(), point.y(), districts.join(", "));
    }
}