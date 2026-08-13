pub mod encryption;
pub mod portable_clipboard;
pub mod portable_input;
pub mod portable_window_tracker;
pub mod repository;
#[cfg(target_os = "windows")]
pub mod windows_ext;

#[cfg(target_os = "windows")]
pub mod windows_api;

/// Non-Windows implementation of the `windows_api` surface.
///
/// Keeps the exact module path and signatures of the Windows implementation so the ~1200
/// call sites need no platform branches, but the clipboard entry points now delegate to
/// `infrastructure::portable_clipboard` (NSPasteboard / X11) instead of silently succeeding
/// as no-ops. Anything still stubbed below either has no portable equivalent (private OLE
/// formats) or is a Windows-only concept.
#[cfg(not(target_os = "windows"))]
pub mod windows_api {
    pub mod win_clipboard {
        use crate::infrastructure::portable_clipboard as portable;

        #[derive(Clone)]
        pub struct ImageData {
            pub width: usize,
            pub height: usize,
            pub bytes: Vec<u8>,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct NamedClipboardFormat {
            pub name: String,
            pub data: Vec<u8>,
        }

        /// Stable between clipboard changes (bumped by the change watcher), restoring the
        /// "same sequence → skip re-capture" debounce that the old always-incrementing stub
        /// defeated.
        pub fn get_clipboard_sequence_number() -> u32 {
            portable::change_sequence()
        }

        pub unsafe fn clear_clipboard() -> Result<(), String> {
            portable::clear()
        }

        pub unsafe fn get_clipboard_image() -> Option<ImageData> {
            portable::get_image_rgba().map(|(width, height, bytes)| ImageData {
                width,
                height,
                bytes,
            })
        }

        pub unsafe fn get_clipboard_files() -> Option<Vec<String>> {
            portable::get_files()
        }

        pub unsafe fn get_clipboard_raw_format(name: &str) -> Option<Vec<u8>> {
            portable::get_raw_format(name)
        }

        /// Application-private formats, identified by UTI (macOS) or selection target (X11)
        /// rather than by Windows format name. They round-trip within a platform, which is
        /// the case that matters: copy from an app and paste back into it.
        pub unsafe fn get_named_clipboard_formats(
            max_formats: usize,
            max_format_bytes: usize,
            max_total_bytes: usize,
            keep: &dyn Fn(&str) -> bool,
        ) -> Vec<NamedClipboardFormat> {
            portable::get_named_formats(max_formats, max_format_bytes, max_total_bytes, keep)
                .into_iter()
                .map(|(name, data)| NamedClipboardFormat { name, data })
                .collect()
        }

        pub unsafe fn set_clipboard_files(paths: Vec<String>) -> Result<(), String> {
            portable::set_files(paths)
        }

        pub unsafe fn set_clipboard_text_and_html(
            text: &str,
            cf_html: &str,
        ) -> Result<(), String> {
            portable::set_rich_content(text, cf_html, None, &[])
        }

        pub unsafe fn append_clipboard_text_and_html(
            text: &str,
            cf_html: &str,
        ) -> Result<(), String> {
            // No append semantics on these pasteboards; a full rewrite is the closest match.
            portable::set_rich_content(text, cf_html, None, &[])
        }

        pub unsafe fn append_named_clipboard_formats(
            _formats: &[NamedClipboardFormat],
        ) -> Result<(), String> {
            Ok(())
        }

        pub unsafe fn set_clipboard_image_and_gif(
            data: ImageData,
            _gif_bytes: Option<&[u8]>,
        ) -> Result<(), String> {
            portable::set_image_rgba(data.width, data.height, &data.bytes)
        }

        /// The `Option<String>` return mirrors the Windows contract, where it carries the
        /// path of a GIF temp file written for CF_HDROP consumers. No such file exists on
        /// these platforms, so it is always `None`.
        pub unsafe fn set_clipboard_image_with_formats(
            data: ImageData,
            _gif_data: Option<&[u8]>,
            _png_data: Option<&[u8]>,
        ) -> Result<Option<String>, String> {
            portable::set_image_rgba(data.width, data.height, &data.bytes)?;
            Ok(None)
        }

        pub unsafe fn set_clipboard_rich_content(
            image_formats: Option<(ImageData, Option<&[u8]>, Option<&[u8]>)>,
            text: &str,
            cf_html: &str,
            named_formats: &[NamedClipboardFormat],
        ) -> Result<Option<String>, String> {
            let image = image_formats
                .as_ref()
                .map(|(image, _, _)| (image.width, image.height, image.bytes.as_slice()));
            let named: Vec<(String, Vec<u8>)> = named_formats
                .iter()
                .map(|f| (f.name.clone(), f.data.clone()))
                .collect();
            portable::set_rich_content(text, cf_html, image, &named)?;
            Ok(None)
        }
    }

    pub mod window_tracker {
        use crate::infrastructure::portable_window_tracker as portable;

        /// Windows installs a WinEvent hook here. macOS/Linux query the frontmost app on
        /// demand instead, so there is nothing to start.
        pub fn start_window_tracking(_app_handle: tauri::AppHandle) {}

        #[derive(Debug, Clone, Default)]
        pub struct ActiveAppInfo {
            pub app_name: String,
            pub process_path: Option<String>,
        }

        fn from_portable(app: portable::ForegroundApp) -> ActiveAppInfo {
            ActiveAppInfo {
                app_name: app.app_name,
                process_path: app.process_path,
            }
        }

        pub fn get_active_app_info() -> ActiveAppInfo {
            from_portable(portable::frontmost_app())
        }

        /// Neither platform exposes a clipboard *owner*, so the frontmost application is the
        /// proxy — the same fallback Windows uses when `GetClipboardOwner` yields nothing.
        pub fn get_clipboard_source_app_info() -> ActiveAppInfo {
            from_portable(portable::frontmost_app())
        }
    }

    pub mod apps {
        use crate::error::AppResult;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct AppInfo {
            pub name: String,
            pub path: String,
        }

        pub async fn launch_uwp_with_file(_package: &str, _file: &str) -> AppResult<()> {
            Ok(())
        }

        #[tauri::command]
        pub fn get_system_default_app(_content_type: String) -> AppResult<String> {
            Ok(String::new())
        }

        #[tauri::command]
        pub fn get_executable_icon(_executable_path: String) -> AppResult<Option<String>> {
            Ok(None)
        }

        #[tauri::command]
        pub fn get_file_icon(_file_path: String) -> AppResult<Option<String>> {
            Ok(None)
        }

        #[tauri::command]
        pub async fn scan_installed_apps() -> AppResult<Vec<AppInfo>> {
            Ok(Vec::new())
        }

        #[tauri::command]
        pub async fn get_associated_apps(_extension: String) -> AppResult<Vec<AppInfo>> {
            Ok(Vec::new())
        }
    }

    pub mod drag_drop {
        pub fn register_emoji_drag_drop(_app_handle: tauri::AppHandle) {}
    }
}
