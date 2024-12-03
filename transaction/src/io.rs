//! A module providing transaction I/O features.

use csv_async::{AsyncDeserializer, AsyncReaderBuilder, AsyncSerializer, AsyncWriterBuilder, Trim};
use futures::stream::TryStreamExt;
use tokio::io;

/// Configure a CSV reader to initiate a transaction process.
pub fn reader(rdr: impl io::AsyncRead + Send + Unpin) -> io::Result<AsyncDeserializer<impl io::AsyncRead>> {
    // let rdr = io::BufReader::new(rdr); // CSV reader is already buffered

    let reader = AsyncReaderBuilder::default()
        .trim(Trim::All)
        .end_on_io_error(true)
        .has_headers(true)
        .flexible(false)
        .create_deserializer(rdr);

    Ok(reader)
}

/// Configure a CSV writer to initiate a transaction process.
pub fn writer(wtr: impl io::AsyncWrite + Unpin) -> io::Result<AsyncSerializer<impl io::AsyncWrite>> {
    let writer = AsyncWriterBuilder::default()
        .has_headers(true)
        .flexible(false)
        .create_serializer(wtr);

    Ok(writer)
}

/// Run a transaction process.
pub async fn process<R, W>(reader: AsyncDeserializer<R>, mut writer: AsyncSerializer<W>) -> crate::Result<()>
where
    R: io::AsyncRead + Send + Unpin,
    W: io::AsyncWrite + Unpin,
{
    let mut stream = reader.into_deserialize::<Vec<String>>();

    while let Some(record) = stream.try_next().await? {
        tracing::debug!("{record:?}");
        writer.serialize(record).await?;
    }

    writer.flush().await?;

    Ok(())
}
