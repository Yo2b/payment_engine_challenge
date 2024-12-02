//! A module providing transaction I/O features.

use tokio::io;

/// Configure a reader to initiate a transaction process.
pub fn reader(rdr: impl io::AsyncRead) -> io::Result<impl io::AsyncRead> {
    let rdr = io::BufReader::new(rdr);

    Ok(rdr)
}

/// Configure a writer to initiate a transaction process.
pub fn writer(wtr: impl io::AsyncWrite) -> io::Result<impl io::AsyncWrite> {
    Ok(wtr)
}

/// Run a transaction process.
pub async fn process<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: io::AsyncRead + Unpin,
    W: io::AsyncWrite + Unpin,
{
    let _ = io::copy(&mut reader, &mut writer).await?;

    Ok(())
}
