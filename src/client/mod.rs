use failure::Error;
use http;
use serde::de::DeserializeOwned;

use super::config::Configuration;

/// APIClient requires `config::Configuration` includes client to connect with kubernetes cluster.
pub struct APIClient {
    configuration: Configuration,
}

type Request<T> = (
    http::Request<Vec<u8>>,
    fn(http::StatusCode) -> k8s_openapi::ResponseBody<k8s_openapi::CreateResponse<T>>,
);
impl APIClient {
    pub fn new(configuration: Configuration) -> Self {
        APIClient { configuration }
    }

    /// Returns kubernetes resources binded `Arnavion/k8s-openapi-codegen` APIs.
    pub fn request<T>(&self, request: Request<T>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let (parts, body) = request.0.into_parts();
        let uri_str = format!("{}{}", self.configuration.base_path, parts.uri);
        let req = match parts.method {
            http::Method::GET => self.configuration.client.get(&uri_str),
            http::Method::POST => self.configuration.client.post(&uri_str),
            http::Method::DELETE => self.configuration.client.delete(&uri_str),
            http::Method::PUT => self.configuration.client.put(&uri_str),
            other => {
                return Err(Error::from(format_err!("Invalid method: {}", other)));
            }
        }
        .body(body);

        let mut response = req.send()?;
        let mut buf: Vec<u8> = vec![];
        response.copy_to(&mut buf)?;

        print!("Status code: {}", response.status().as_u16());
        let mut response =
            request.1(http::status::StatusCode::from_u16(response.status().as_u16()).unwrap());
        response.append_slice(&buf);

        let response = response.parse()?;

        match response {
            k8s_openapi::CreateResponse::Ok(job) => Ok(job),
            k8s_openapi::CreateResponse::Created(job) => Ok(job),
            k8s_openapi::CreateResponse::Accepted(job) => Ok(job),
            k8s_openapi::CreateResponse::Other(result) => match result {
                Ok(value) => Err(failure::err_msg(serde_json::to_string(&value).unwrap())),
                Err(err) => Err(failure::Error::from(err)),
            },
        }

        // response.json().map_err(Error::from)
    }
}
