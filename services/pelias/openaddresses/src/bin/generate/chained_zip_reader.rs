use crate::Result;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::ops::Range;
use std::path::Path;
use zip::ZipArchive;

/// Reads all files in a ZipArchive
///
/// To avoid running into OS limits, only one file is kept open at a time.
pub struct ChainedZipReader {
    archive: ZipArchive<BufReader<File>>,
    files: Range<usize>,
    current_file: Option<Cursor<Vec<u8>>>,
}

impl ChainedZipReader {
    pub fn new(path: &Path) -> Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        let archive = ZipArchive::new(reader)?;

        Ok(Self {
            files: 0..archive.len(),
            archive,
            current_file: None,
        })
    }
}

impl std::io::Read for ChainedZipReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut current_file = match self.current_file.take() {
            Some(current_file) => current_file,
            None => {
                let Some(next) = self.files.next() else {
                    return Ok(0);
                };
                // yikes. I'm not sure how to own the archive while cursing through it's files
                // so lets just read the entire file into memory (only one at a time though, which
                // should be fine for our use case)
                let mut file = self.archive.by_index(next)?;
                debug!("reading {file_name}", file_name = file.name());
                let mut file_contents = vec![];
                file.read_to_end(&mut file_contents)?;
                Cursor::new(file_contents)
            }
        };

        let size = current_file.read(buf)?;
        if size == 0 {
            self.current_file = None;
            // recurse to advance to next file in archive
            self.read(buf)
        } else {
            self.current_file = Some(current_file);
            Ok(size)
        }
    }
}
