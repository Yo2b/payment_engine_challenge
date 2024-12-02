//! A module providing transaction I/O features.

use csv_async::{AsyncReader, AsyncReaderBuilder, AsyncWriter, AsyncWriterBuilder, Error, Trim};
use futures::stream::TryStreamExt;
use tokio::io;

/// Configure a CSV reader to initiate a transaction process.
pub fn reader(rdr: impl io::AsyncRead + Send + Unpin) -> io::Result<AsyncReader<impl io::AsyncRead>> {
    // let rdr = io::BufReader::new(rdr); // CSV reader is already buffered

    let reader = AsyncReaderBuilder::default()
        .trim(Trim::All)
        .end_on_io_error(true)
        .has_headers(true)
        .flexible(false)
        .create_reader(rdr);

    Ok(reader)
}

/// Configure a CSV writer to initiate a transaction process.
pub fn writer(wtr: impl io::AsyncWrite + Unpin) -> io::Result<AsyncWriter<impl io::AsyncWrite>> {
    let writer = AsyncWriterBuilder::default()
        .has_headers(true)
        .flexible(false)
        .create_writer(wtr);

    Ok(writer)
}

/// Run a transaction process.
pub async fn process<R, W>(mut reader: AsyncReader<R>, mut writer: AsyncWriter<W>) -> Result<(), Error>
where
    R: io::AsyncRead + Send + Unpin,
    W: io::AsyncWrite + Unpin,
{
    let headers = reader.byte_headers().await?;
    writer.write_byte_record(headers).await?;

    let mut stream = reader.byte_records();
    while let Some(record) = stream.try_next().await? {
        tracing::debug!("{record:?}");
        writer.write_byte_record(&record).await?;
    }

    writer.flush().await?;

    Ok(())
}
