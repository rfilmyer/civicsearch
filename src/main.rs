use std::process;
use std::collections::HashMap;
use shapefile::dbase::FieldValue;
use shapefile::Polygon;
use geo::algorithm::contains::Contains;

fn main() {
    let reader = shapefile::Reader::from_path("tl_2019_25_sldl/tl_2019_25_sldl.shp")
        .unwrap_or_else(|err| {
            eprintln!("Problem reading shapefile: {}", err);
            process::exit(1);
    });
    let lat = 42.389408;
    let lon = -71.119699;

    let point = geo_types::Point::new(lon, lat);

    let matching_districts = reader
        .iter_shapes_and_records_as::<Polygon>()
        .unwrap_or_else(|err| {eprintln!("Problem reading shapefile: {}", err); process::exit(1)})
        .filter_map(|r| r.ok())
        .filter(|r| district_contains_point(r, &point))
        .map(|r| get_district_name(r))
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

fn district_contains_point<T>(district: &(Polygon, T), point: &geo_types::Point<f64>) -> bool {
    let (shape, _) = district;
    let shape = shape.clone();
    //let shape = geo_types::MultiPolygon::<f64>::from(shape);
    let shape: geo_types::MultiPolygon<f64> =  shape.into();

    // false
    shape.contains(point)
}

fn get_district_name<T>(district: (T, HashMap<String, FieldValue>)) -> Option<String> {
    let (_, record) = district;
    match record.get("NAMELSAD") {
        Some(FieldValue::Character(Some(n))) => Some(n.clone()),
        _ => None, 
    }
}