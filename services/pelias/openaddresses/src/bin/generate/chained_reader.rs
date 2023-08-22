use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// Chain multiple files together as a single reader.
///
/// To avoid running into OS limits, only one file is kept open at a time.
pub struct ChainedReader {
    paths: Vec<PathBuf>,
    current: Option<BufReader<File>>,
}

impl ChainedReader {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            current: None,
        }
    }
}

impl std::io::Read for ChainedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.current {
            Some(current) => match current.read(buf)? {
                0 => {
                    self.current = None;
                    self.read(buf)
                }
                read_len => Ok(read_len),
            },
            None => match self.paths.pop() {
                None => Ok(0),
                Some(next) => {
                    log::debug!("Opening next reader: {next:?}");
                    self.current = Some(BufReader::new(File::open(next)?));
                    self.read(buf)
                }
            },
        }
    }
}
