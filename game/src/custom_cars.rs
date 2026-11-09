//! Local OBJ + PNG replacements. Stock assets are retained for instant restoration.
use crate::{assets::Assets, model::Model, persist};
use drdriving_core::cars::{car, PLAYER_CARS};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

const MAX_FILE: usize = 8 * 1024 * 1024;

/// The web pack ships known detailed cars separately from the stock bundle.
#[cfg(target_arch="wasm32")]
pub async fn load_bundled(a:&mut Assets) {
    let manifest=crate::assets::present_while(macroquad::file::load_file("assets/cars/catalog.json")).await
        .expect("Detailed car catalog could not be loaded");
    let catalog:serde_json::Value=serde_json::from_slice(&manifest).expect("Invalid car catalog");
    let cars=catalog.as_array().expect("Car catalog must be a list");
    assert!(cars.len()<=PLAYER_CARS,"Car catalog is too large");
    for (index,item) in cars.iter().enumerate() {
        let slot=item["slot"].as_u64().expect("Car slot") as usize;
        assert!(slot<PLAYER_CARS,"Invalid car slot");
        let path=item["path"].as_str().expect("Car package path");
        assert!(path.starts_with("assets/cars/")&&!path.contains(".."),"Invalid car package path");
        let name=item["name"].as_str().unwrap_or("Modern car");
        let label=format!("Loading modern cars {}/{} — {}",index+1,cars.len(),name);
        let logo=a.logo.clone();let font=a.font.clone();let pct=((index+1)*100/cars.len()) as i32;
        crate::assets::set_loading_draw(Some(Box::new(move||{
            crate::ui_menu::draw_loading(&mut crate::gfx::Gfx::new(),&logo,&crate::gfx::GlText{font:font.clone()},pct,1);
            crate::draw_label(&label,&font);
        })));
        let request_path=path.to_string();
        let bytes=crate::assets::present_while(async move {macroquad::file::load_file(&request_path).await}).await
            .unwrap_or_else(|e|panic!("Cannot load {path}: {e}"));
        let detailed=crate::car_render::DetailedCar::from_bytes(&bytes,slot)
            .unwrap_or_else(|e|panic!("Detailed car {slot}: {e}"));
        a.custom_car_name[slot]=Some(detailed.name.clone());
        a.custom_car[slot]=true;
        a.detailed_cars[slot]=Some(detailed);
        next_frame().await;
    }
}
type Files = (Vec<u8>, Vec<u8>);
type PickResult = Result<Option<Files>, String>;

pub struct CustomCars {
    originals: Vec<Option<(Model, Texture2D)>>,
    pending: Option<(usize, Receiver<PickResult>)>,
}

fn path(slot: usize) -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    let base = persist::save_path().with_file_name("custom-cars");
    #[cfg(target_arch = "wasm32")]
    let base = PathBuf::from("custom-cars");
    base.join(format!("car{slot:02}.car"))
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > limit { return Err("Car files are too large (8 MB per file).".into()); }
    Ok(bytes)
}

fn decode(slot: usize, obj: &[u8], png: &[u8]) -> Result<(Model, Texture2D), String> {
    if obj.len() > MAX_FILE || png.len() > MAX_FILE { return Err("Use files smaller than 8 MB each.".into()); }
    let text = std::str::from_utf8(obj).map_err(|_| "OBJ must be UTF-8 text.")?;
    let info = car(slot);
    let model = Model::import_obj(text, info.half_width, info.half_length, info.eye_height * 1.25)?;
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return Err("Choose a valid PNG texture.".into());
    }
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    if width == 0 || height == 0 || width > 2048 || height > 2048 {
        return Err("PNG texture must be at most 2048 x 2048 pixels.".into());
    }
    let img = Image::from_file_with_format(png, Some(ImageFormat::Png)).map_err(|_| "The PNG could not be decoded.")?;
    let tex = Texture2D::from_image(&img);
    tex.set_filter(FilterMode::Linear);
    Ok((model, tex))
}

impl CustomCars {
    pub fn load(a: &mut Assets) -> Self {
        let mut result = Self { originals: (0..PLAYER_CARS).map(|_| None).collect(), pending: None };
        #[cfg(not(target_arch = "wasm32"))]
        for slot in 0..PLAYER_CARS {
            let detailed_path = path(slot).with_extension("carhq");
            if detailed_path.exists() {
                match crate::car_render::DetailedCar::load(&detailed_path, slot) {
                    Ok(detailed) => {
                        a.custom_car_name[slot] = Some(detailed.name.clone());
                        a.custom_car[slot] = true;
                        a.detailed_cars[slot] = Some(detailed);
                    }
                    Err(e) => eprintln!("Detailed car {slot}: {e}; using standard replacement."),
                }
            }
            if a.detailed_cars[slot].is_some() { continue; }
            if !path(slot).exists() { continue; }
            let load = || -> Result<(Model, Texture2D), String> {
                let bytes = read_limited(&path(slot), MAX_FILE * 2 + 4)?;
                if bytes.len() < 4 { return Err("Incomplete car file.".into()); }
                let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                if len > MAX_FILE || len > bytes.len() - 4 { return Err("Invalid car file.".into()); }
                decode(slot, &bytes[4..4 + len], &bytes[4 + len..])
            };
            match load() {
                Ok((model, tex)) => {
                    result.install(a, slot, model, tex);
                    a.custom_car_name[slot] = read_limited(&path(slot).with_extension("name"), 256).ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .map(|name| name.trim().chars().filter(|c| !c.is_control()).take(32).collect::<String>())
                        .filter(|name| !name.is_empty());
                }
                Err(e) => eprintln!("Skipping custom car {slot}: {e}"),
            }
        }
        result
    }

    fn install(&mut self, a: &mut Assets, slot: usize, model: Model, tex: Texture2D) {
        let old = (std::mem::replace(&mut a.car_model[slot], model), std::mem::replace(&mut a.car_tex[slot], tex));
        if self.originals[slot].is_none() { self.originals[slot] = Some(old); }
        a.custom_car[slot] = true;
        a.custom_car_name[slot] = None;
    }

    pub fn busy(&self) -> bool { self.pending.is_some() }

    pub fn begin(&mut self, slot: usize) -> Result<(), String> {
        if slot >= PLAYER_CARS { return Err("Choose a playable garage car.".into()); }
        if self.busy() { return Err("Finish choosing the current car files first.".into()); }
        if persist::READ_ONLY.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Imports are unavailable in screenshot/demo mode.".into());
        }
        let (tx, rx) = mpsc::channel();
        let owner = picker_owner();
        std::thread::spawn(move || { let _ = tx.send(pick_files(owner)); });
        self.pending = Some((slot, rx));
        Ok(())
    }

    pub fn poll(&mut self, a: &mut Assets) -> Option<String> {
        let (slot, rx) = self.pending.as_ref()?;
        let slot = *slot;
        let picked = match rx.try_recv() {
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("The file picker closed unexpectedly.".into()),
            Ok(picked) => picked,
        };
        self.pending = None;
        let files = match picked {
            Ok(Some(files)) => files,
            Ok(None) => return Some("Import canceled; your car is unchanged.".into()),
            Err(e) => return Some(e),
        };
        match self.import(a, slot, files) {
            Ok(()) => Some(format!("Custom model imported for {}.", car(slot).name)),
            Err(e) => Some(e),
        }
    }

    fn import(&mut self, a: &mut Assets, slot: usize, files: Files) -> Result<(), String> {
        let (obj, png) = files;
        let (model, tex) = decode(slot, &obj, &png)?;
        let dest = path(slot);
        let dir = dest.parent().ok_or("Cannot locate the custom car folder.")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create custom car folder: {e}"))?;
        let mut bytes = Vec::with_capacity(4 + obj.len() + png.len());
        bytes.extend_from_slice(&(obj.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&obj);
        bytes.extend_from_slice(&png);
        let tmp = dest.with_extension("tmp");
        std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, &dest))
            .map_err(|e| format!("Cannot save custom car: {e}"))?;
        // New manual imports should not inherit the installed pack's display name.
        if let Err(e) = std::fs::remove_file(dest.with_extension("name")) {
            if e.kind() != std::io::ErrorKind::NotFound { eprintln!("Cannot clear custom car name: {e}"); }
        }
        self.install(a, slot, model, tex);
        a.detailed_cars[slot] = None;
        if let Err(e) = std::fs::remove_file(dest.with_extension("carhq")) {
            if e.kind() != std::io::ErrorKind::NotFound { eprintln!("Cannot clear detailed car: {e}"); }
        }
        Ok(())
    }

    pub fn restore(&mut self, a: &mut Assets, slot: usize) -> Result<String, String> {
        if self.busy() { return Err("Finish choosing car files first.".into()); }
        if self.originals[slot].is_none() && a.detailed_cars[slot].is_none() { return Ok("This car already uses its original model.".into()); }
        match std::fs::remove_file(path(slot).with_extension("carhq")) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(format!("Cannot remove detailed car: {e}")),
        }
        match std::fs::remove_file(path(slot)) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(format!("Cannot remove custom car: {e}")),
        }
        if let Some((model, tex)) = self.originals[slot].take() {
            a.car_model[slot] = model;
            a.car_tex[slot] = tex;
        }
        a.detailed_cars[slot] = None;
        a.custom_car[slot] = false;
        a.custom_car_name[slot] = None;
        if let Err(e) = std::fs::remove_file(path(slot).with_extension("name")) {
            if e.kind() != std::io::ErrorKind::NotFound { eprintln!("Cannot clear custom car name: {e}"); }
        }
        Ok("Original car restored.".into())
    }
}

#[cfg(target_os = "windows")]
fn picker_owner() -> usize {
    // Capture the game window on its UI thread before opening the dialog on a worker.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as usize }
}

#[cfg(not(target_os = "windows"))]
fn picker_owner() -> usize { 0 }

#[cfg(target_os = "windows")]
fn pick_files(owner: usize) -> PickResult {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let choose = |title: &str, filter: &str| -> Result<Option<PathBuf>, String> {
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let filter: Vec<u16> = filter.encode_utf16().collect();
        let mut file = vec![0u16; 32768];
        // All-zero pointers/callbacks are valid for the optional OPENFILENAMEW fields.
        let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = owner as _;
        dialog.lpstrTitle = title.as_ptr();
        dialog.lpstrFilter = filter.as_ptr();
        dialog.nFilterIndex = 1;
        dialog.lpstrFile = file.as_mut_ptr();
        dialog.nMaxFile = file.len() as u32;
        dialog.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR;
        // Buffers and the owner handle stay alive for the entire synchronous native call.
        if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
            let error = unsafe { CommDlgExtendedError() };
            return if error == 0 { Ok(None) } else { Err(format!("File picker error: {error:#x}")) };
        }
        let len = file.iter().position(|v| *v == 0).unwrap_or(file.len());
        Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(&file[..len]))))
    };
    let Some(obj) = choose("Import car: select the triangulated OBJ model", "Wavefront OBJ (*.obj)\0*.obj\0\0")? else { return Ok(None) };
    let Some(png) = choose("Import car: select its PNG texture (max 2048 x 2048)", "PNG texture (*.png)\0*.png\0\0")? else { return Ok(None) };
    Ok(Some((read_limited(&obj, MAX_FILE)?, read_limited(&png, MAX_FILE)?)))
}

#[cfg(not(target_os = "windows"))]
fn pick_files(_: usize) -> PickResult { Err("The car file picker is currently available on Windows.".into()) }
