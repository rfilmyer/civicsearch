use std::io::{self, Seek};
use std::fs::File;
use std::collections::HashMap;
use shapefile::Reader;
use shapefile::dbase::FieldValue;
use shapefile::Polygon;
use geo::algorithm::contains::Contains;
use zip::{ZipArchive};
use std::ffi::OsStr;
use tempfile::tempfile;
use log::{debug, info};

/// Checks if a point is in a shape.
/// 
/// This is a utility function that explicitly converts a 
/// shapefile::Polygon into a geo_types::MultiPolygon<f64>
/// in order to explicitly do the check. 
/// In a brighter future, this function can be eliminated and replaced with a single line in a closure somewhere.
/// 
/// # Examples
/// 
/// ```
/// use shapefile::{Polygon, PolygonRing, Point};
/// 
/// let simple_polygon = Polygon::new(PolygonRing::Outer(vec![
///     Point::new(-1.1,-1.01),
///     Point::new(-1.2, 1.02),
///     Point::new( 1.3, 1.03),
///     Point::new( 1.4,-1.04),
/// ]));
/// 
/// let inside_point = geo_types::Point::new(0.0, 0.0);
/// 
/// assert_eq!(civicsearch::shape_contains_point(&simple_polygon, &inside_point), true);
/// ```
/// 
/// ```
/// # use shapefile::{Polygon, PolygonRing, Point};
/// 
/// let simple_polygon = Polygon::new(PolygonRing::Outer(vec![
///     Point::new(-1.1,-1.01),
///     Point::new(-1.2, 1.02),
///     Point::new( 1.3, 1.03),
///     Point::new( 1.4,-1.04),
/// ]));
/// 
/// let outside_point = geo_types::Point::new(2.0, 0.0);
/// 
/// assert_eq!(civicsearch::shape_contains_point(&simple_polygon, &outside_point), false);
/// ```
/// 
/// ```
/// # use shapefile::{Polygon, PolygonRing, Point};
/// 
/// # let simple_polygon = Polygon::new(PolygonRing::Outer(vec![
/// #    Point::new(-1.1,-1.01),
/// #    Point::new(-1.2, 1.02),
/// #    Point::new( 1.3, 1.03),
/// #    Point::new( 1.4,-1.04),
/// # ]));
/// 
/// let on_edge_point = geo_types::Point::new(1.1, -1.01);
/// 
/// assert_eq!(civicsearch::shape_contains_point(&simple_polygon, &on_edge_point), true);
/// ```
pub fn shape_contains_point(shape: &Polygon, point: &geo_types::Point<f64>) -> bool {
    let shape: geo_types::MultiPolygon<f64> =  shape.clone().into();
    shape.contains(point)
}

/// Searches a shape's record to find the name of a district.
/// 
/// For TIGER shapefiles, district names are stored in the "NAMELSAD" field 
/// in the `.dbf` database stored with a .shp shapefile.
/// 
/// # Examples
/// ```
/// use std::collections::HashMap;
/// use shapefile::dbase::FieldValue;
/// 
/// let mut record = HashMap::new();
/// 
/// record.insert(String::from("NAMELSAD"), 
///     FieldValue::Character(
///         Some(String::from("1st District")))
/// );
/// 
/// assert_eq!(civicsearch::extract_district_name(record), Some(String::from("1st District")));
/// ```
/// 
/// ```
/// # use std::collections::HashMap;
/// 
/// let mut empty_record = HashMap::new();
/// 
/// assert_eq!(civicsearch::extract_district_name(empty_record), None);
/// ```
pub fn extract_district_name(record: HashMap<String, FieldValue>) -> Option<String> {
    match record.get("NAMELSAD") {
        Some(FieldValue::Character(Some(n))) => Some(n.clone()),
        _ => None, 
    }
}

struct TIGERShapefileArchive<T> 
where T: io::Read,
{
    shape_file:      T,
    db_file:         T,
    shapeindex_file: T,
}

fn copy_into_new_tempfile(mut old_file: impl io::Read) -> Result<File, io::Error> {
    debug!("Creating tempfile");
    let mut temp_file = tempfile()?;
    debug!("Extracting zip file into temp file");
    io::copy(&mut old_file, &mut temp_file)?;
    debug!("Copy complete");
    temp_file.seek(io::SeekFrom::Start(0))?;
    Ok(temp_file)
}

fn extract_shapefiles<'a, R: 'a>(mut zip_archive: ZipArchive<R>) -> Result<TIGERShapefileArchive<File>, io::Error>
    where R: io::Read + io::Seek,
{
    let mut shape_file = None; 
    let mut db_file = None; 
    let mut shapeindex_file = None;

    info!("Opening Zip Archive");
    for i in 0..zip_archive.len() {
        let file = zip_archive.by_index(i)?;
        let file_name = file.sanitized_name();
        debug!("Found File {}", file_name.to_string_lossy());
        let file_extension = file_name.extension()
            .unwrap_or_else(|| { OsStr::new("") })
            .to_str();
        match file_extension {
                Some("shp") => {
                    info!("Extracting .shp into tempfile");
                    shape_file = Some(copy_into_new_tempfile(file)?);
                    debug!("Extracted .shp into tempfile");
                }
                Some("dbf") => {
                    info!("Extracting .dbf into tempfile");
                    db_file = Some(copy_into_new_tempfile(file)?);
                    debug!("Extracted .dbf into tempfile");
                }
                Some("shx") => {
                    info!("Extracting .shx into tempfile");
                    shapeindex_file = Some(copy_into_new_tempfile(file)?);
                    debug!("Extracted .shx into tempfile");
                }
                _ => {}
            }
    }

    Ok(
        TIGERShapefileArchive::<File>{
            shape_file: shape_file.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing .shp file in archive"))?,
            db_file: db_file.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing .dbf file in archive"))?,
            shapeindex_file: shapeindex_file.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing .shx file in archive"))?
    })
}


/// Reads a zip archive (a `.zip` file) from TIGER and returns a `shapefile::Reader` to be used for further munging.
/// 
/// TIGER shapefiles come in zip archives with a standard format.  
/// For example, take the 2019 Massachusetts State Assembly district file `tl_2019_25_sldl.zip`:
/// 
/// ```text
/// tl_2019_25_sldl.zip
/// |- tl_2019_25_sldl.cpg
/// |- tl_2019_25_sldl.dbf
/// |- tl_2019_25_sldl.prj
/// |- tl_2019_25_sldl.shp
/// |- tl_2019_25_sldl.shx
/// |- tl_2019_25_sldl.shp.ea.iso.xml
/// |- tl_2019_25_sldl.shp.iso.xml
/// ```
/// We need three of these files, the `.shp`, the `.dbf`, and the `.shx`, 
/// so it is probably more user-friendly to look for a single file, versus asking for three.
/// This function builds a `shapefile::Reader` from a zip file containing those three files.
/// 
/// # Undefined Behavior
/// The function expects a zipfile with exactly one `.shp` file, one `.dbf` file, and one `.shx` file,
/// as is typical for TIGER shapefiles. It is unclear what happens if there are multiple of those files in an archive.
/// 
/// # Errors
/// 
pub fn shapefile_reader_from_zip_archive<R>(zip_archive: ZipArchive<R>) -> Result<Reader<impl io::Read>, shapefile::Error> 
    where R: io::Read + io::Seek,
{
    let tiger_shapefile = extract_shapefiles(zip_archive)?;
    debug!("Creating Reader");
    let mut reader = Reader::new(tiger_shapefile.shape_file)?;
    debug!("Adding .dbf");
    reader.add_dbf_source(tiger_shapefile.db_file)?;
    debug!("Adding .shx");
    reader.add_index_source(tiger_shapefile.shapeindex_file)?;

    Ok(reader)
    
    
}