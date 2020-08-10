use std::process;
use std::fs::File;
use shapefile::Polygon;
use zip::{ZipArchive};
use log::{error};

fn main() {
    env_logger::init();
    // let reader = shapefile::Reader::from_path("tl_2019_25_sldl/tl_2019_25_sldl.shp")
    //     .unwrap_or_else(|err| {
    //         eprintln!("Problem reading shapefile: {}", err);
    //         process::exit(1);
    // });
    let zipfile_path = "tl_2019_25_sldl.zip";
    let zipfile_file = File::open(zipfile_path)
        .unwrap_or_else(|err| {error!("Problem reading archive: {:?}", err); process::exit(1)});
    let zipfile_archive = ZipArchive::new(zipfile_file)
        .unwrap_or_else(|err| {error!("Problem reading zip file: {:?}", err); process::exit(1)});
    let reader = civicsearch::shapefile_reader_from_zip_archive(zipfile_archive)
        .unwrap_or_else(|err| {error!("Problem parsing shapefile: {:?}", err); process::exit(1)});

    let lat = 42.389408;
    let lon = -71.119699;

    let point = geo_types::Point::new(lon, lat);

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