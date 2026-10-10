use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSPrintInfo, NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSPrinter, NSPrintingPaginationMode,
    NSWindow,
};
use objc2_foundation::{NSCopying, NSObject, NSObjectProtocol, NSSize, NSString, NSURL};
use objc2_web_kit::WKWebView;
use std::cell::RefCell;
use std::ffi::c_void;
use tauri::async_runtime::Sender;
use tauri::webview::PlatformWebview;
use tauri::WebviewWindow;

const A4_WIDTH_POINTS: f64 = 595.276;
const A4_HEIGHT_POINTS: f64 = 841.89;

type Outcome = Result<(), String>;

pub struct PrintIvars {
    sender: RefCell<Option<Sender<Outcome>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AquilumPdfPrintDelegate"]
    #[ivars = PrintIvars]
    struct PrintDelegate;

    unsafe impl NSObjectProtocol for PrintDelegate {}

    impl PrintDelegate {
        #[unsafe(method(printOperationDidRun:success:contextInfo:))]
        fn did_run(&self, _operation: &NSPrintOperation, success: Bool, _context: *mut c_void) {
            let outcome = if success.as_bool() {
                Ok(())
            } else {
                Err("macOS не смог записать PDF по этому пути".to_owned())
            };
            if let Some(sender) = self.ivars().sender.borrow_mut().take() {
                let _ = sender.try_send(outcome);
            }
            ACTIVE.with(|active| active.borrow_mut().take());
        }
    }
);

thread_local! {
    static ACTIVE: RefCell<Option<Retained<PrintDelegate>>> = const { RefCell::new(None) };
}

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

fn start(webview: &PlatformWebview, path: &str, sender: Sender<Outcome>) -> Outcome {
    let main_thread = MainThreadMarker::new().ok_or("Печать в PDF должна идти из главного потока")?;
    let web_view = unsafe { &*(webview.inner() as *const WKWebView) };
    let window = unsafe { &*(webview.ns_window() as *const NSWindow) };

    let info = print_info(path);
    let operation = unsafe { web_view.printOperationWithPrintInfo(&info) };
    operation.setShowsPrintPanel(false);
    operation.setShowsProgressPanel(true);
    operation.setCanSpawnSeparateThread(true);
    if let Some(view) = operation.view() {
        view.setFrame(web_view.bounds());
    }

    let delegate = PrintDelegate::alloc(main_thread).set_ivars(PrintIvars { sender: RefCell::new(Some(sender)) });
    let delegate: Retained<PrintDelegate> = unsafe { msg_send![super(delegate), init] };
    ACTIVE.with(|active| *active.borrow_mut() = Some(delegate.clone()));
    unsafe {
        operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
            window,
            Some(&delegate),
            Some(sel!(printOperationDidRun:success:contextInfo:)),
            std::ptr::null_mut(),
        );
    }
    Ok(())
}

fn use_any_printer_type_without_printers(info: &NSPrintInfo) {
    if NSPrinter::printerNames().count() > 0 {
        return;
    }
    let Some(kind) = NSPrinter::printerTypes().firstObject() else { return };
    if let Some(printer) = NSPrinter::printerWithType(&kind) {
        info.setPrinter(&printer);
    }
}

fn print_info(path: &str) -> Retained<NSPrintInfo> {
    let info = NSPrintInfo::sharedPrintInfo().copy();
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    unsafe {
        info.setJobDisposition(NSPrintSaveJob);
        let key: &NSString = NSPrintJobSavingURL;
        let value: &AnyObject = url.as_ref();
        info.dictionary().setObject_forKey(value, ProtocolObject::from_ref(key));
    }
    info.setPaperSize(NSSize::new(A4_WIDTH_POINTS, A4_HEIGHT_POINTS));
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
    info.setVerticalPagination(NSPrintingPaginationMode::Automatic);
    info.setHorizontallyCentered(false);
    info.setVerticallyCentered(false);
    use_any_printer_type_without_printers(&info);
    info.setUpPrintOperationDefaultValues();
    info
}
