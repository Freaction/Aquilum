use tauri::webview::PlatformWebview;
use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Environment6, ICoreWebView2_2, ICoreWebView2_7,
};
use webview2_com::PrintToPdfCompletedHandler;
use windows::core::{Interface, HSTRING};

const A4_WIDTH_INCHES: f64 = 8.267_716_5;
const A4_HEIGHT_INCHES: f64 = 11.692_913_4;

type Outcome = Result<(), String>;

pub async fn print_current_page(window: WebviewWindow, path: String) -> Outcome {
    let (sender, mut receiver) = tauri::async_runtime::channel::<Outcome>(1);
    let failure = sender.clone();
    window
        .with_webview(move |webview| {
            if let Err(error) = start(&webview, &path, sender) {
                let _ = failure.try_send(Err(error));
            }
        })
        .map_err(|error| error.to_string())?;

    receiver
        .recv()
        .await
        .unwrap_or_else(|| Err("Печать завершилась без ответа".to_owned()))
}

fn start(
    webview: &PlatformWebview,
    path: &str,
    sender: tauri::async_runtime::Sender<Outcome>,
) -> Outcome {
    let file = HSTRING::from(path);
    unsafe {
        let core = webview
            .controller()
            .CoreWebView2()
            .map_err(describe)?;
        let environment = core
            .cast::<ICoreWebView2_2>()
            .map_err(describe)?
            .Environment()
            .map_err(describe)?
            .cast::<ICoreWebView2Environment6>()
            .map_err(describe)?;

        let settings = environment.CreatePrintSettings().map_err(describe)?;
        settings.SetPageWidth(A4_WIDTH_INCHES).map_err(describe)?;
        settings.SetPageHeight(A4_HEIGHT_INCHES).map_err(describe)?;
        settings.SetMarginTop(0.0).map_err(describe)?;
        settings.SetMarginBottom(0.0).map_err(describe)?;
        settings.SetMarginLeft(0.0).map_err(describe)?;
        settings.SetMarginRight(0.0).map_err(describe)?;
        settings.SetShouldPrintBackgrounds(true).map_err(describe)?;
        settings
            .SetShouldPrintHeaderAndFooter(false)
            .map_err(describe)?;

        let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, saved| {
            let outcome = match (result, saved) {
                (Ok(()), true) => Ok(()),
                (Ok(()), false) => Err("WebView2 не смог записать PDF по этому пути".to_owned()),
                (Err(error), _) => Err(describe(error)),
            };
            let _ = sender.try_send(outcome);
            Ok(())
        }));

        core.cast::<ICoreWebView2_7>()
            .map_err(describe)?
            .PrintToPdf(&file, &settings, &handler)
            .map_err(describe)?;
    }
    Ok(())
}

fn describe(error: windows::core::Error) -> String {
    format!("WebView2: {error}")
}
