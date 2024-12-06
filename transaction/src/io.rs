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
    let stream = crate::Processor::process(reader.into_deserialize().err_into());
    tokio::pin!(stream);

    while let Some(record) = stream.try_next().await? {
        writer.serialize(record).await?;
    }

    writer.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_process(input: &[u8], output: &mut Vec<u8>) -> crate::Result<()> {
        let buffer = std::io::Cursor::new(output);

        let reader = AsyncDeserializer::from_reader(input);
        let writer = AsyncSerializer::from_writer(buffer);

        process(reader, writer).await
    }

    #[tokio::test/* (flavor = "multi_thread") */]
    #[tracing_test::traced_test]
    async fn test_process_ok() {
        let transactions = r"
type,client,tx,amount
deposit,1,1,5.1
deposit,1,2,0.2
deposit,1,3,1.0
withdrawal,1,4,4.2
dispute,1,2,
resolve,1,2,
dispute,1,3,
chargeback,1,3,
";

        let mut data = vec![];
        test_process(transactions.as_bytes(), &mut data).await.unwrap();
        assert_eq!(data, b"client,available,held,total,locked\n1,1.1,0,1.1,true\n");
    }
}
