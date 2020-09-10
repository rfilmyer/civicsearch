use std::io::{Read, Seek, Cursor};
use std::collections::HashMap;
use shapefile::Reader;
use shapefile::dbase::FieldValue;
use shapefile::Polygon;
use geo::algorithm::contains::Contains;
use zip::{ZipArchive, read::ZipFile};
use std::path::Path;
use std::ffi::OsStr;

use log::{debug, info};

use thiserror::Error;

/// Errors that can occur when reading TIGER shapefiles
#[derive(Error, Debug)]
pub enum TIGERShapefileError {

    /// One of three expected files (`.shp`, `.dbf`, or `.shx`) is missing from the TIGER shapefile `.zip` archive
    #[error("required .{extension:} file not found in archive")]
    MissingFile {
        extension: String,
    },
    
    /// There are multiple `.shp`, `.dbf`, or `.shx` files in the TIGER shapefile `.zip` archive (there should only be one of each)
    #[error("too many .{extension:} files in archive")]
    TooManyFiles {
        extension: String,
    },

    /// There was an error of some kind in reading the TIGER shapefile `.zip` archive
    #[error("error reading zipfile")]
    ZipFile(#[from] zip::result::ZipError),

    /// There was an error in processing or parsing the TIGER shapefile
    #[error("error processing shapefile")]
    InvalidShapefile(#[from] shapefile::Error),

    /// There was some kind of IO error (likely in `.zip` extraction)
    #[error("IO Error")]
    Io(#[from] std::io::Error),
}

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

/// Stores the filenames of relevant data within a TIGER shapefile archive
struct TIGERShapefileArchiveFilenames<'a> {
    shp_filename: &'a str,
    dbf_filename: &'a str,
    shx_filename: &'a str,
}

/// 
fn find_file_in_zipfile_by_extension<'a, R>(zip_archive: &'a ZipArchive<R>, extension: &str) -> Result<Option<&'a str>, TIGERShapefileError> 
    where R: Read + Seek,
{
    let filenames_with_extension = zip_archive.file_names()
        .filter(|f| {
            Path::new(f)
                .extension()
                .and_then(|x| { Some(OsStr::to_string_lossy(x)) })
                .map_or(false, |x| { x == extension})
        })
        .collect::<Vec<&str>>();
    
    if filenames_with_extension.len() > 1 {
        return Err(TIGERShapefileError::TooManyFiles { extension: String::from(extension) })
    } else {
        return Ok(filenames_with_extension.first().cloned())
    }
}

/// Searches in the zipped TIGER shapefile for relevant files by extension
/// 
/// There are three files we need from the zipped TIGER shapefile - 
/// (cf [section 2.2.1 of the TIGER Shapefile technical document](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2019/TGRSHP2019_TechDoc.pdf))
/// * .shp - the feature geometry
/// * .shx - the index of the feature geometry
/// * .dbf - the tabular attribute information
fn get_shapefile_names_from_tiger_zipfile<R>(zip_archive: &ZipArchive<R>) -> Result<TIGERShapefileArchiveFilenames, TIGERShapefileError> 
    where R: Read + Seek,
{
    Ok(TIGERShapefileArchiveFilenames{
        shp_filename: find_file_in_zipfile_by_extension(zip_archive, "shp")?
            .ok_or(TIGERShapefileError::MissingFile { extension: String::from("shp")})?,
        dbf_filename: find_file_in_zipfile_by_extension(zip_archive, "dbf")?
            .ok_or(TIGERShapefileError::MissingFile { extension: String::from("dbf")})?,
        shx_filename: find_file_in_zipfile_by_extension(zip_archive, "shx")?
        .ok_or(TIGERShapefileError::MissingFile { extension: String::from("shx")})?,
    })
}

/// Represents the actual files (or file-like objects) from a zipped TIGER shapefile
struct TIGERShapefileArchive<T> 
where T: Read,
{
    shape_file:      T,
    db_file:         T,
    shapeindex_file: T,
}

fn extract_file_in_memory(mut zip_file: ZipFile) -> Result<Cursor<Vec<u8>>, TIGERShapefileError> 
{
    let mut output_buffer = Vec::new();
    zip_file.read_to_end(&mut output_buffer)?;
    Ok(Cursor::new(output_buffer))
}

fn extract_shapefiles<'a, R: 'a>(zip_archive: ZipArchive<R>) -> Result<TIGERShapefileArchive<Cursor<Vec<u8>>>, TIGERShapefileError>
    where R: Clone + Read + Seek,
{
    info!("Checking for Files in Archive");
    let shapefile_names = get_shapefile_names_from_tiger_zipfile(&zip_archive)?;

    Ok(
        TIGERShapefileArchive {
            shape_file:      extract_file_in_memory(zip_archive.clone().by_name(shapefile_names.shp_filename)?)?,
            db_file:         extract_file_in_memory(zip_archive.clone().by_name(shapefile_names.dbf_filename)?)?,
            shapeindex_file: extract_file_in_memory(zip_archive.clone().by_name(shapefile_names.shx_filename)?)?,
        }
    )
}


/// Reads a zip archive (a `.zip` file) from TIGER and returns a `shapefile::Reader` to be used for further munging.
/// 
/// TIGER shapefiles come in zip archives with a standard format (cf [section 2.2.1 of the TIGER Shapefile technical document](https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2019/TGRSHP2019_TechDoc.pdf))  
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
/// 
/// # Errors
/// This function will return an error if: 
/// * One of the 3 required files is not present in the `.zip` archive
/// * There are multiple `.shp`, `.dbf`, or `.shx` files in the archive
/// * There is an error opening the zip file (see errors in the `zip` crate for more details)
/// * There is an error parsing the `.shp`, `.dbf`, or `.shx` files (see `shapefile::Error` for more details)
/// 
pub fn shapefile_reader_from_zip_archive<R>(zip_archive: ZipArchive<R>) -> Result<Reader<impl Read>, TIGERShapefileError> 
    where R: Clone + Read + Seek,
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