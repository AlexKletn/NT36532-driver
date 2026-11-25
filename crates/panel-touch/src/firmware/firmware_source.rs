use std::{
    fs::File,
    io::{self, ErrorKind},
    os::{fd::BorrowedFd, unix::fs::FileExt},
    path::Path,
    vec::Vec,
};

pub fn read_firmware_path(path: &Path) -> io::Result<Vec<u8>> {
    read_file(&File::open(path)?)
}

pub fn read_firmware_fd(fd: BorrowedFd<'_>) -> io::Result<Vec<u8>> {
    read_file(&File::from(fd.try_clone_to_owned()?))
}

fn read_file(file: &File) -> io::Result<Vec<u8>> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "Firmware input must be a regular file",
        ));
    }
    let size = usize::try_from(metadata.len())
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(io::Error::other)?;
    bytes.resize(size, 0);
    // Positional reads preserve the cursor of the caller's file descriptor.
    file.read_exact_at(&mut bytes, 0)?;
    Ok(bytes)
}
