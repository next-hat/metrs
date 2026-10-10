use ntex::channel::mpsc::Receiver;

use metrs_stubs::*;

use crate::client::MetrsdClient;
use crate::error::{ApiError, MetrsClientError, is_api_error};

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
    let client = MetrsdClient::connect("http://127.0.0.1:8080")
      .await
      .unwrap();
    let mut stream = client.subscribe().await.unwrap();
    let mut count = 0;
    const MAX_COUNT: usize = 5;
    while let Some(event) = stream.next().await {
      let event = event.unwrap();
      println!("{event:?}");
      count += 1;
      if count == MAX_COUNT {
        break;
      }
    }
    assert_eq!(count, MAX_COUNT)
  }

  #[cfg(unix)]
  #[ntex::test]
  async fn test_unix_subscribe_and_api_error() {
    use std::{
      os::unix::net::UnixListener,
      sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
      },
      time::{SystemTime, UNIX_EPOCH},
    };

    use ntex::{SharedCfg, http::StatusCode, util::Bytes, web};

    let directory = std::env::temp_dir().join(format!(
      "metrs-client-{}-{}",
      std::process::id(),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos(),
    ));
    std::fs::create_dir(&directory).unwrap();
    let socket = directory.join("http.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let expected = MetrsdEvent {
      memory: MemoryInfo {
        total: 1024,
        free: 768,
        used: 256,
        swap_total: 512,
        swap_free: 512,
        swap_used: 0,
      },
      cpus: Vec::new(),
      disks: Vec::new(),
      networks: Vec::new(),
    };
    let body =
      Bytes::from(format!("{}\n", serde_json::to_string(&expected).unwrap(),));
    let reject = Arc::new(AtomicBool::new(false));
    let server_reject = reject.clone();
    let server = web::HttpServer::new(async move |_| {
      let reject = server_reject.clone();
      let body = body.clone();
      web::App::new().route(
        "/subscribe",
        web::get().to(async move || {
          if reject.load(Ordering::SeqCst) {
            web::HttpResponse::BadRequest().json(&serde_json::json!({
              "msg": "subscription rejected",
            }))
          } else {
            web::HttpResponse::Ok().streaming(futures::stream::iter([Ok::<
              _,
              std::io::Error,
            >(
              body.clone(),
            )]))
          }
        }),
      )
    })
    .workers(1)
    .disable_signals()
    .listen_uds(listener, SharedCfg::default())
    .unwrap()
    .run();

    let url =
      Box::leak(format!("unix://{}", socket.display()).into_boxed_str());
    let client = MetrsdClient::connect(url).await.unwrap();
    let outcome = async {
      let mut stream = client.subscribe().await?;
      let event = stream.next().await;
      let end = stream.next().await;
      reject.store(true, Ordering::SeqCst);
      let error = client.subscribe().await.err();
      Ok::<_, MetrsClientError>((event, end, error))
    }
    .await;

    server.stop(false).await;
    std::fs::remove_dir_all(&directory).unwrap();

    let (event, end, error) = outcome.unwrap();
    let event = event.unwrap().unwrap();
    assert_eq!(
      serde_json::to_value(event).unwrap(),
      serde_json::to_value(expected).unwrap(),
    );
    assert!(end.is_none());
    let MetrsClientError::Api(error) = error.unwrap() else {
      panic!("expected API error from Unix subscription");
    };
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.msg, "subscription rejected");
  }
}
