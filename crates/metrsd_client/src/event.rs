use metrs_stubs::*;
use ntex::channel::mpsc::Receiver;

use crate::client::MetrsdClient;
use crate::error::{ApiError, MetrsClientError, is_api_error};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
#[serde(tag = "Type", content = "Data")]
pub enum MetrsdEvent {
  Memory(MemoryInfo),
  Cpu(Vec<CpuInfo>),
  Disk(Vec<DiskInfo>),
  Network(Vec<NetworkInfo>),
}

impl MetrsdClient {
  pub async fn subscribe(
    &self,
  ) -> Result<Receiver<Result<MetrsdEvent, ApiError>>, MetrsClientError> {
    let mut res = self.get("/subscribe".to_string()).send().await?;
    let status = res.status();
    is_api_error(&mut res, &status).await?;

    Ok(self.stream(res))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use futures::StreamExt;

  #[ntex::test]
  async fn test_subscribe() {
    let client = MetrsdClient::connect("http://127.0.0.1:8080");

    let mut stream = client.subscribe().await.unwrap();

    let mut count = 0;
    const MAX_COUNT: usize = 50;

    while let Some(event) = stream.next().await {
      println!("{:?}", event);
      count += 1;
      if count == MAX_COUNT {
        break;
      }
    }
    assert_eq!(count, MAX_COUNT)
  }
}
