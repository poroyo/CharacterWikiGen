use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, FormData, RequestInit, RequestMode, Request, Response};
use shared_constants::PROXY;

#[wasm_bindgen]
pub async fn upload_file(file_data: Vec<u8>, file_name: String) -> Result<String, JsValue> {
    let url = format!("{}https://catbox.moe/user/api.php", PROXY);
    let blob_parts: js_sys::Array = js_sys::Array::new();
    let file_bytes = js_sys::Uint8Array::from(file_data.as_slice());
    blob_parts.push(&file_bytes);
    let file_blob = Blob::new_with_u8_array_sequence(&blob_parts)?;

    let window = web_sys::window().unwrap();

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);

    let form_data = FormData::new()?;
    form_data.append_with_str("reqtype", "fileupload").unwrap();
    form_data.append_with_str("userhash", "").unwrap();
    form_data.set_with_blob_and_filename("fileToUpload", &file_blob, &file_name)?;

    opts.body(Some(&form_data));

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response_value.dyn_into().unwrap();

    if response.ok() {
        let response_text = JsFuture::from(response.text()?).await?;
        Ok(response_text.as_string().unwrap())
    } else {
        Err(JsValue::from_str("Error uploading file"))
    }
}
