use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "ssr"))]
pub fn fetch_api<T>(
    path: &str,
) -> impl std::future::Future<Output = Option<T>> + Send + '_
where
    T: Serialize + DeserializeOwned,
{
    use leptos::prelude::on_cleanup;
    use send_wrapper::SendWrapper;
    use leptos::logging::log;

    SendWrapper::new(async move {
        let abort_controller =
            SendWrapper::new(web_sys::AbortController::new().ok());
        let abort_signal = abort_controller.as_ref().map(|a| a.signal());

        // abort in-flight requests if, e.g., we've navigated away from this page
        on_cleanup(move || {
            if let Some(abort_controller) = abort_controller.take() {
                abort_controller.abort()
            }
        });

        gloo_net::http::Request::post(path)
            .header("Access-Control-Allow-Origin", "http://127.0.0.1:3002")
            .abort_signal(abort_signal.as_ref())
            .send()
            .await
            .map_err(|e| log!("{e}"))
            .ok()?
            .json()
            .await
            .ok()
    })
}



#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serialize + DeserializeOwned,
{
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", "http://127.0.0.1:3002".parse().unwrap());
    let client = reqwest::Client::builder().default_headers(headers).build().unwrap();
    client.post(path)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()
}