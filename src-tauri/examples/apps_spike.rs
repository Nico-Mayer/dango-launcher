//! M1 spike: enumerate the shell applications folder, extract icons, and
//! launch by AppUserModelID. Feeds the `applications` extension design.
//!
//! Usage: `cargo run --example apps_spike -- [list | icons <dir> [filter] | launch <aumid>]`

#[cfg(not(windows))]
fn main() {
    eprintln!("apps_spike only runs on Windows");
}

#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    spike::run()
}

#[cfg(windows)]
mod spike {
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;
    use std::time::Instant;

    use windows::core::{w, Interface, Result, HSTRING, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{E_FAIL, SIZE};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::Storage::EnhancedStorage::{
        PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, IEnumShellItems, IShellItem, IShellItem2, IShellItemImageFactory,
        SHCreateItemFromParsingName, ShellExecuteW, SIGDN_DESKTOPABSOLUTEPARSING,
        SIGDN_NORMALDISPLAY, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    const ICON_SIZE: i32 = 64;

    struct Entry {
        name: String,
        aumid: String,
        target: Option<String>,
        parsing: String,
        item: IShellItem,
    }

    impl Entry {
        fn kind(&self) -> &'static str {
            if self.aumid.contains('!') {
                "packaged"
            } else if self.target.is_some() {
                "win32"
            } else {
                "other"
            }
        }
    }

    pub fn run() -> Result<()> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            Some("icons") => icons(
                args.get(1).map_or("spike-icons", String::as_str),
                args.get(2).map(String::as_str),
            ),
            Some("launch") => launch(args.get(1).expect("usage: launch <aumid>")),
            _ => list(),
        }
    }

    fn list() -> Result<()> {
        let started = Instant::now();
        let entries = enumerate()?;
        let elapsed = started.elapsed();
        for e in &entries {
            println!(
                "{:<9} {:<40} {:<75} {}",
                e.kind(),
                e.name,
                e.aumid,
                e.target.as_deref().unwrap_or(&e.parsing)
            );
        }
        let count = |k: &str| entries.iter().filter(|e| e.kind() == k).count();
        println!(
            "\n{} entries in {:.1}ms: packaged={} win32={} other={}",
            entries.len(),
            elapsed.as_secs_f64() * 1000.0,
            count("packaged"),
            count("win32"),
            count("other")
        );
        Ok(())
    }

    fn icons(dir: &str, filter: Option<&str>) -> Result<()> {
        std::fs::create_dir_all(dir).expect("create output dir");
        let entries = enumerate()?;
        let selected = entries
            .iter()
            .filter(|e| filter.is_none_or(|f| e.name.to_lowercase().contains(&f.to_lowercase())));
        let (mut ok, mut failed) = (0, 0);
        let total = Instant::now();
        for e in selected {
            let started = Instant::now();
            match extract_icon(&e.item) {
                Ok((w, h, rgba)) => {
                    write_png(
                        &Path::new(dir).join(format!("{}.png", sanitize(&e.name))),
                        w,
                        h,
                        &rgba,
                    );
                    ok += 1;
                    println!(
                        "{:>7.1}ms {}x{} {:<9} {}",
                        started.elapsed().as_secs_f64() * 1000.0,
                        w,
                        h,
                        e.kind(),
                        e.name
                    );
                }
                Err(err) => {
                    failed += 1;
                    println!("   FAIL {:<9} {}: {err}", e.kind(), e.name);
                }
            }
        }
        println!(
            "\n{ok} icons, {failed} failures, {:.0}ms total",
            total.elapsed().as_secs_f64() * 1000.0
        );
        Ok(())
    }

    fn launch(aumid: &str) -> Result<()> {
        let file = HSTRING::from(format!("shell:AppsFolder\\{aumid}"));
        let started = Instant::now();
        let instance = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                &file,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecute reports success as any value above 32.
        println!(
            "ShellExecuteW returned {} in {:.1}ms",
            instance.0 as isize,
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(())
    }

    fn enumerate() -> Result<Vec<Entry>> {
        unsafe {
            let folder: IShellItem = SHCreateItemFromParsingName(w!("shell:AppsFolder"), None)?;
            let items: IEnumShellItems = folder.BindToHandler(None, &BHID_EnumItems)?;
            let mut entries = Vec::new();
            loop {
                let mut slot = [None];
                let mut fetched = 0;
                items.Next(&mut slot, Some(&mut fetched))?;
                let Some(item) = slot[0].take().filter(|_| fetched > 0) else {
                    break;
                };
                let item2: IShellItem2 = item.cast()?;
                let name = take_string(item.GetDisplayName(SIGDN_NORMALDISPLAY)?);
                let parsing = take_string(item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING)?);
                let aumid = item2
                    .GetString(&PKEY_AppUserModel_ID)
                    .map(|p| take_string(p))
                    .unwrap_or_default();
                let target = item2
                    .GetString(&PKEY_Link_TargetParsingPath)
                    .ok()
                    .map(|p| take_string(p));
                entries.push(Entry {
                    name,
                    aumid,
                    target,
                    parsing,
                    item,
                });
            }
            Ok(entries)
        }
    }

    unsafe fn take_string(p: PWSTR) -> String {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as _));
        s
    }

    fn extract_icon(item: &IShellItem) -> Result<(u32, u32, Vec<u8>)> {
        unsafe {
            let factory: IShellItemImageFactory = item.cast()?;
            let hbitmap: HBITMAP = factory.GetImage(
                SIZE {
                    cx: ICON_SIZE,
                    cy: ICON_SIZE,
                },
                SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
            )?;
            let rgba = bitmap_to_rgba(hbitmap);
            let _ = DeleteObject(hbitmap.into());
            rgba
        }
    }

    unsafe fn bitmap_to_rgba(hbitmap: HBITMAP) -> Result<(u32, u32, Vec<u8>)> {
        let mut bitmap = BITMAP::default();
        GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut BITMAP as *mut _),
        );
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative height requests top-down rows.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            hbitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        if lines == 0 {
            return Err(E_FAIL.into());
        }
        // GDI hands back premultiplied BGRA; PNG wants straight RGBA.
        for px in pixels.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255) / a).min(255) as u8;
                }
            }
        }
        Ok((width as u32, height as u32, pixels))
    }

    fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        let file = File::create(path).expect("create png");
        let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .expect("png header")
            .write_image_data(rgba)
            .expect("png data");
    }

    fn sanitize(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }
}
