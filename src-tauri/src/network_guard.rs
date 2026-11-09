use tauri::Manager;

pub fn allowed(url: &tauri::Url) -> bool {
    crate::ipc_security::trusted_location(url)
        || (url.scheme() == "http"
            && url.host_str() == Some("ipc.localhost")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none())
}

#[cfg(windows)]
pub fn install(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use webview2_com::{
        take_pwstr, Microsoft::Web::WebView2::Win32::*, WebResourceRequestedEventHandler,
    };
    use windows::core::{HSTRING, PWSTR};
    let app = window.app_handle().clone();
    window.with_webview(move |platform| {
        let result = (|| -> windows::core::Result<()> {
            // SAFETY: interfaces COM pertencem à thread de WebView fornecida por with_webview.
            unsafe {
                let webview = platform.controller().CoreWebView2()?;
                let environment = platform.environment();
                webview.AddWebResourceRequestedFilter(
                    &HSTRING::from("http://*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                )?;
                webview.AddWebResourceRequestedFilter(
                    &HSTRING::from("https://*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                )?;
                let mut token = 0;
                webview.add_WebResourceRequested(
                    &WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                        if let Some(args) = args {
                            let request = args.Request()?;
                            let mut uri = PWSTR::null();
                            request.Uri(&mut uri)?;
                            let text = take_pwstr(uri);
                            if !tauri::Url::parse(&text).is_ok_and(|url| allowed(&url)) {
                                let response = environment.CreateWebResourceResponse(
                                    None,
                                    403,
                                    &HSTRING::from("Blocked"),
                                    &HSTRING::from(
                                        "Content-Type: text/plain\r\nCache-Control: no-store\r\n",
                                    ),
                                )?;
                                args.SetResponse(&response)?;
                            }
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }
            Ok(())
        })();
        if result.is_err() {
            eprintln!("Astock: não foi possível proteger a rede da WebView. Atualize o WebView2.");
            app.exit(1);
        }
    })
}

#[cfg(not(windows))]
pub fn install(_: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resources_are_limited_to_internal_origins() {
        for text in [
            "http://tauri.localhost/assets/app.js",
            "http://ipc.localhost/get_workspace",
        ] {
            assert!(allowed(&tauri::Url::parse(text).unwrap()));
        }
        for text in [
            "https://example.invalid/",
            "http://127.0.0.1:9999/",
            "http://ipc.localhost.evil.invalid/",
            "http://user:pass@ipc.localhost/",
            "file:///C:/Windows/win.ini",
        ] {
            assert!(!allowed(&tauri::Url::parse(text).unwrap()));
        }
    }
}
