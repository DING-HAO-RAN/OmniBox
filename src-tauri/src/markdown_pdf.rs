use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 与前端 PDF 设置面板对应的导出参数。
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PdfSettings {
    pub page_size: String,
    pub orientation: String,
    pub custom_width_mm: f64,
    pub custom_height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_right_mm: f64,
    pub margin_bottom_mm: f64,
    pub margin_left_mm: f64,
    pub base_font_family: String,
    pub base_font_size_pt: f64,
    pub line_height: f64,
    pub paragraph_spacing_pt: f64,
    pub heading_scale: f64,
    pub text_color: String,
    pub link_color: String,
    pub code_font_family: String,
    pub code_theme: String,
    pub image_max_width_percent: f64,
    pub show_backgrounds: bool,
    pub show_header_footer: bool,
    pub header_text: String,
    pub footer_text: String,
    pub show_page_number: bool,
    pub scale_factor: f64,
}

#[derive(Clone, Debug)]
struct NativePrintValues {
    page_width_in: f64,
    page_height_in: f64,
    margin_top_in: f64,
    margin_right_in: f64,
    margin_bottom_in: f64,
    margin_left_in: f64,
    scale_factor: f64,
    landscape: bool,
    print_backgrounds: bool,
    print_header_footer: bool,
    header_title: String,
}

fn millimeters_to_inches(value_mm: f64) -> f64 {
    value_mm / 25.4
}

fn page_dimensions_mm(settings: &PdfSettings) -> Result<(f64, f64), String> {
    let (width_mm, height_mm) = match settings.page_size.as_str() {
        "a3" => (297.0, 420.0),
        "a4" => (210.0, 297.0),
        "a5" => (148.0, 210.0),
        "letter" => (215.9, 279.4),
        "legal" => (215.9, 355.6),
        "custom" => (settings.custom_width_mm, settings.custom_height_mm),
        _ => return Err("不支持的 PDF 页面尺寸。".to_string()),
    };

    if !width_mm.is_finite() || !height_mm.is_finite() || width_mm < 50.0 || height_mm < 50.0 {
        return Err("PDF 页面尺寸必须是至少 50 mm 的有效数值。".to_string());
    }

    if settings.orientation == "landscape" {
        Ok((height_mm, width_mm))
    } else if settings.orientation == "portrait" {
        Ok((width_mm, height_mm))
    } else {
        Err("不支持的 PDF 页面方向。".to_string())
    }
}

fn validate_settings(settings: &PdfSettings) -> Result<NativePrintValues, String> {
    let (page_width_mm, page_height_mm) = page_dimensions_mm(settings)?;
    let margins = [
        settings.margin_top_mm,
        settings.margin_right_mm,
        settings.margin_bottom_mm,
        settings.margin_left_mm,
    ];
    if margins
        .iter()
        .any(|margin| !margin.is_finite() || *margin < 0.0)
    {
        return Err("页边距必须是非负的有效数值。".to_string());
    }
    if settings.margin_left_mm + settings.margin_right_mm >= page_width_mm
        || settings.margin_top_mm + settings.margin_bottom_mm >= page_height_mm
    {
        return Err("页边距总和必须小于页面尺寸。".to_string());
    }
    if !settings.base_font_size_pt.is_finite()
        || !(6.0..=48.0).contains(&settings.base_font_size_pt)
    {
        return Err("基础字号必须在 6–48 pt 之间。".to_string());
    }
    if !settings.line_height.is_finite() || !(1.0..=3.0).contains(&settings.line_height) {
        return Err("行高必须在 1–3 之间。".to_string());
    }
    if !settings.paragraph_spacing_pt.is_finite()
        || !(0.0..=48.0).contains(&settings.paragraph_spacing_pt)
    {
        return Err("段后距必须在 0–48 pt 之间。".to_string());
    }
    if !settings.heading_scale.is_finite() || !(0.6..=1.6).contains(&settings.heading_scale) {
        return Err("标题缩放必须在 0.6–1.6 之间。".to_string());
    }
    if !settings.image_max_width_percent.is_finite()
        || !(25.0..=100.0).contains(&settings.image_max_width_percent)
    {
        return Err("图片最大宽度必须在 25–100% 之间。".to_string());
    }
    if !settings.scale_factor.is_finite() || !(0.5..=1.5).contains(&settings.scale_factor) {
        return Err("打印缩放必须在 50%–150% 之间。".to_string());
    }

    Ok(NativePrintValues {
        page_width_in: millimeters_to_inches(page_width_mm),
        page_height_in: millimeters_to_inches(page_height_mm),
        margin_top_in: millimeters_to_inches(settings.margin_top_mm),
        margin_right_in: millimeters_to_inches(settings.margin_right_mm),
        margin_bottom_in: millimeters_to_inches(settings.margin_bottom_mm),
        margin_left_in: millimeters_to_inches(settings.margin_left_mm),
        scale_factor: settings.scale_factor,
        landscape: settings.orientation == "landscape",
        print_backgrounds: settings.show_backgrounds,
        print_header_footer: settings.show_page_number,
        header_title: settings.header_text.clone(),
    })
}

fn validate_output_path(output_path: &str) -> Result<PathBuf, String> {
    let requested_path = PathBuf::from(output_path);
    if !requested_path.is_absolute() {
        return Err("PDF 输出路径必须是绝对路径。".to_string());
    }
    if requested_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        != Some(true)
    {
        return Err("PDF 输出文件必须使用 .pdf 扩展名。".to_string());
    }

    let parent = requested_path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .ok_or_else(|| "PDF 输出路径缺少有效目录。".to_string())?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(|error| format!("无法访问 PDF 输出目录：{error}"))?;
    let file_name = requested_path
        .file_name()
        .ok_or_else(|| "PDF 输出路径缺少文件名。".to_string())?;
    Ok(canonical_parent.join(file_name))
}

/// 用当前主窗口 WebView2 的 PrintToPdf 接口输出 PDF。
#[tauri::command]
pub async fn export_markdown_pdf(
    app: tauri::AppHandle,
    output_path: String,
    settings: PdfSettings,
) -> Result<(), String> {
    let print_values = validate_settings(&settings)?;
    let output_path = validate_output_path(&output_path)?;

    #[cfg(windows)]
    {
        export_windows_pdf(app, output_path, print_values).await
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        let _ = output_path;
        let _ = print_values;
        Err("当前平台没有可用的 WebView2 PDF 导出引擎。".to_string())
    }
}

#[cfg(windows)]
async fn export_windows_pdf(
    app: tauri::AppHandle,
    output_path: PathBuf,
    print_values: NativePrintValues,
) -> Result<(), String> {
    use std::sync::mpsc::sync_channel;
    use tauri::Manager;

    let webview_window = app
        .get_webview_window("main")
        .ok_or_else(|| "找不到主窗口，无法启动 PDF 导出。".to_string())?;
    let (sender, receiver) = sync_channel::<Result<(), String>>(1);
    let callback_sender = sender.clone();
    let print_output_path = output_path.clone();
    let dispatch_result = webview_window.with_webview(move |webview| {
        let start_sender = callback_sender.clone();
        if let Err(error) =
            start_print_to_pdf(webview, &print_output_path, print_values, start_sender)
        {
            let _ = callback_sender.send(Err(error));
        }
    });

    if let Err(error) = dispatch_result {
        let _ = sender.send(Err(format!("无法访问 WebView2：{error}")));
    }

    let print_result = tauri::async_runtime::spawn_blocking(move || {
        receiver
            .recv_timeout(Duration::from_secs(60))
            .map_err(|_| "PDF 导出超时，WebView2 没有返回完成状态。".to_string())?
    })
    .await
    .map_err(|error| format!("PDF 导出任务异常：{error}"))?;
    print_result?;

    let metadata = fs::metadata(&output_path)
        .map_err(|error| format!("PDF 已报告成功，但文件不可访问：{error}"))?;
    if metadata.len() == 0 {
        return Err("PDF 已报告成功，但输出文件为空。".to_string());
    }

    Ok(())
}

#[cfg(windows)]
fn start_print_to_pdf(
    webview: tauri::webview::PlatformWebview,
    output_path: &Path,
    print_values: NativePrintValues,
    completion_sender: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment6, ICoreWebView2_7, COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE,
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT,
    };
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{Interface, PCWSTR};

    let core_webview = unsafe { webview.controller().CoreWebView2() }
        .map_err(|error| format!("获取 CoreWebView2 实例失败：{error}"))?;
    let webview7 = core_webview
        .cast::<ICoreWebView2_7>()
        .map_err(|error| format!("当前 WebView2 不支持 PrintToPdf：{error}"))?;
    let environment = webview
        .environment()
        .cast::<ICoreWebView2Environment6>()
        .map_err(|error| format!("当前 WebView2 不支持打印设置：{error}"))?;
    let print_settings = unsafe { environment.CreatePrintSettings() }
        .map_err(|error| format!("创建 WebView2 打印设置失败：{error}"))?;

    let orientation = if print_values.landscape {
        COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE
    } else {
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT
    };
    unsafe {
        print_settings
            .SetOrientation(orientation)
            .map_err(|error| format!("设置页面方向失败：{error}"))?;
        print_settings
            .SetPageWidth(print_values.page_width_in)
            .map_err(|error| format!("设置页面宽度失败：{error}"))?;
        print_settings
            .SetPageHeight(print_values.page_height_in)
            .map_err(|error| format!("设置页面高度失败：{error}"))?;
        print_settings
            .SetMarginTop(print_values.margin_top_in)
            .map_err(|error| format!("设置上边距失败：{error}"))?;
        print_settings
            .SetMarginRight(print_values.margin_right_in)
            .map_err(|error| format!("设置右边距失败：{error}"))?;
        print_settings
            .SetMarginBottom(print_values.margin_bottom_in)
            .map_err(|error| format!("设置下边距失败：{error}"))?;
        print_settings
            .SetMarginLeft(print_values.margin_left_in)
            .map_err(|error| format!("设置左边距失败：{error}"))?;
        print_settings
            .SetScaleFactor(print_values.scale_factor)
            .map_err(|error| format!("设置打印缩放失败：{error}"))?;
        print_settings
            .SetShouldPrintBackgrounds(print_values.print_backgrounds)
            .map_err(|error| format!("设置背景打印选项失败：{error}"))?;
        print_settings
            .SetShouldPrintHeaderAndFooter(print_values.print_header_footer)
            .map_err(|error| format!("设置页码选项失败：{error}"))?;
    }

    let output_string = output_path.to_string_lossy().into_owned();
    let output_wide: Vec<u16> = output_string
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let header_wide: Vec<u16> = print_values
        .header_title
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let empty_wide = [0u16];
    let completion_handler =
        PrintToPdfCompletedHandler::create(Box::new(move |error_code, succeeded| {
            let result = match error_code {
                Err(error) => Err(format!("WebView2 PDF 打印失败：{error}")),
                Ok(()) if succeeded => Ok(()),
                Ok(()) => Err("WebView2 PDF 打印未成功完成。".to_string()),
            };
            let _ = completion_sender.send(result);
            Ok(())
        }));

    unsafe {
        // 自定义页眉页脚主体由打印 CSS 控制；空 URI 可避免 WebView2 显示应用地址。
        print_settings
            .SetHeaderTitle(PCWSTR(header_wide.as_ptr()))
            .map_err(|error| format!("设置打印页眉标题失败：{error}"))?;
        print_settings
            .SetFooterUri(PCWSTR(empty_wide.as_ptr()))
            .map_err(|error| format!("清理打印页脚地址失败：{error}"))?;
        webview7
            .PrintToPdf(
                PCWSTR(output_wide.as_ptr()),
                &print_settings,
                &completion_handler,
            )
            .map_err(|error| format!("启动 WebView2 PDF 打印失败：{error}"))?;
    }

    Ok(())
}
