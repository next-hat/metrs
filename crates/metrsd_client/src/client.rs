use futures::{StreamExt, TryStreamExt};
use ntex::{
  ServiceFactory,
  channel::mpsc::Receiver,
  client::{Client, ClientRequest, ClientResponse, Connector},
  http::StatusCode,
  rt,
};

use crate::error::ApiError;

#[derive(Clone)]
pub struct MetrsdClient {
  client: Client,
  url: String,
}

impl MetrsdClient {
  pub async fn connect(url: &'static str) -> Result<Self, ApiError> {
    match url {
      url if url.starts_with("http://") || url.starts_with("https://") => {
        let client = Client::builder()
          .response_timeout(ntex::time::Millis::from_secs(20))
          .build(ntex::SharedCfg::default())
          .await
          .map_err(|err| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: format!("Unable to create client got error : {err}"),
          })?;
        Ok(MetrsdClient {
          client,
          url: url.to_owned(),
        })
      }
      url if url.starts_with("unix://") => {
        let unix_socket = url.trim_start_matches("unix://").to_owned();
        let mut client = Client::builder()
          .response_payload_timeout(ntex::time::Millis::from_secs(20))
          .response_timeout(ntex::time::Millis::from_secs(20));
        client = client.connector::<&str>(
          Connector::default().connector(
            ntex::service::fn_service(move |_| {
              let unix_socket = unix_socket.clone();
              async move {
                rt::unix_connect(unix_socket, ntex::SharedCfg::default())
                  .await
                  .map_err(ntex::connect::ConnectError::from)
              }
            })
            .map_init_err(|_| {
              ntex::connect::ConnectServiceError::CannotCreateService
            }),
          ),
        );
        let client =
          client
            .build(ntex::SharedCfg::default())
            .await
            .map_err(|err| ApiError {
              status: StatusCode::INTERNAL_SERVER_ERROR,
              msg: format!("Unable to create client got error : {err}"),
            })?;
        Ok(MetrsdClient {
          client,
          url: String::from("http://localhost"),
        })
      }
      url => {
        panic!("Invalid url valid scheme are [http,https,unix] got: {url}");
      }
    }
  }

  pub(crate) fn get(&self, url: String) -> ClientRequest {
    self.client.get(self.gen_url(url))
  }

  fn gen_url(&self, url: String) -> String {
    self.url.to_owned() + &url
  }

  pub(crate) fn stream<T>(
    &self,
    res: ClientResponse,
  ) -> Receiver<Result<T, ApiError>>
  where
    T: serde::de::DeserializeOwned + Send + 'static,
  {
    let mut stream = res.into_stream();
    let (tx, rx) = ntex::channel::mpsc::channel();
    rt::spawn(async move {
      let mut payload: Vec<u8> = Vec::new();
      while let Some(item) = stream.next().await {
        let bytes = match item {
          Ok(bytes) => bytes,
          Err(e) => {
            let _ = tx.send(Err(ApiError {
              status: StatusCode::INTERNAL_SERVER_ERROR,
              msg: format!("Unable to read stream got error : {e}"),
            }));
            break;
          }
        };
        payload.extend(bytes.to_vec());
        if bytes.last() != Some(&b'\n') {
          continue;
        }
        let t = match serde_json::from_slice::<T>(&payload) {
          Ok(t) => t,
          Err(e) => {
            let _ = tx.send(Err(ApiError {
              status: StatusCode::INTERNAL_SERVER_ERROR,
              msg: format!("Unable to parse stream got error : {e}"),
            }));
            break;
          }
        };
        payload.clear();
        if tx.send(Ok(t)).is_err() {
          break;
        }
      }
      tx.close();
    });
    rx
  }
}

#[cfg(test)]
mod tests {
  use crate::error::is_api_error;

  use super::*;

  #[ntex::test]
  async fn test_new_client() {
    let client = MetrsdClient::connect("http://unknow.internal")
      .await
      .unwrap();
    assert_eq!(client.url, "http://unknow.internal");
    let res = client.subscribe().await;
    assert!(res.is_err());
    let client = MetrsdClient::connect("https://unknow.internal")
      .await
      .unwrap();
    assert_eq!(client.url, "https://unknow.internal");
    let res = client.subscribe().await;
    assert!(res.is_err());
    let client = MetrsdClient::connect("unix:///run/_non_existent.sock")
      .await
      .unwrap();
    assert_eq!(client.url, "http://localhost");
    let res = client.subscribe().await;
    assert!(res.is_err());
  }

  #[ntex::test]
  #[should_panic]
  async fn test_new_client_wrong_scheme() {
    let _ = MetrsdClient::connect("ftp://domain.com").await;
  }

  #[ntex::test]
  async fn test_gen_url() {
    let client = MetrsdClient::connect("http://domain.com").await.unwrap();
    assert_eq!(
      client.gen_url("/test".to_string()),
      "http://domain.com/test"
    );
  }

  #[ntex::test]
  async fn test_wrong_get() {
    let client = MetrsdClient::connect("http://321313131").await.unwrap();
    let res = client.get("/test".to_string()).send().await;
    assert!(res.is_err());
  }

  #[ntex::test]
  async fn test_api_error() {
    let client = MetrsdClient::connect("http://127.0.0.1:8080")
      .await
      .unwrap();
    let mut res = client.get("/test".to_string()).send().await.unwrap();
    let status = res.status();
    let err = is_api_error(&mut res, &status).await;
    println!("{err:?}");
    assert!(err.is_err());
    let err = err.unwrap_err();
    println!("{err}");
  }
}
