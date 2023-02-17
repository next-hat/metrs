use ntex::web;
use ntex::http::StatusCode;

#[derive(Debug)]
pub enum MetrsError {
  Error(String),
}

impl std::fmt::Display for MetrsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MetrsError::Error(err) => write!(f, "{err}"),
    }
  }
}

#[derive(Debug)]
pub struct HttpError {
  pub status: StatusCode,
  pub msg: String,
}

impl std::fmt::Display for HttpError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{status}]: {msg}", status = self.status, msg = self.msg)
  }
}

impl ntex::web::WebResponseError for HttpError {
  // Builds the actual response to send back when an error occurs
  fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
    log::error!("{self}");
    let err_json = serde_json::json!({ "msg": self.msg });
    web::HttpResponse::build(self.status).json(&err_json)
  }
}
