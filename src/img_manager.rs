use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::{Rc, Weak};
use anyhow::Error;
use image::{DynamicImage, EncodableLayout, ImageReader};
use libc::memcpy;
use skia_safe::{AlphaType, Bitmap, ColorSpace, ColorType, Image, ImageInfo};
use std::borrow::Borrow;
use std::cell::RefCell;

thread_local! {
    pub static IMG_MANAGER: ImgManager = ImgManager::new();
}

pub struct ImgManager {
    cache: RefCell<HashMap<String, Weak<Image>>>
}

impl ImgManager {

    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }
    pub fn load_img(&self, path: &str) -> Result<Image, Error> {
        if let Some(cached_img) = self.cache.borrow().get(path) {
            if let Some(img) = cached_img.upgrade() {
                let i: &Image = img.borrow();
                let ii = i.clone();
                return Ok(ii);
            }
        }
        let img = ImageReader::open(path)?.decode()?;
        let sk_img = dyn_image_to_skia_image(&img);
        let rc = Rc::new(sk_img.clone());
        self.cache.borrow_mut().insert(path.to_string(), Rc::downgrade(&rc));
        Ok(sk_img)
    }

    pub fn get_img(&self, src: &str) -> Option<Image> {
        let img_res = self.load_img(src);
        match img_res {
            Ok(img) => {
                Some(img)
            }
            Err(err) => {
                println!("failed to load image:{:?}", err);
                None
            }
        }
    }
}

pub fn dyn_image_to_skia_image(src: &DynamicImage) -> Image {
    let width = src.width() as i32;
    let height = src.height() as i32;
    let image_info = ImageInfo::new((width, height), ColorType::RGBA8888, AlphaType::Unpremul, ColorSpace::new_srgb());
    let mut bm = Bitmap::new();
    let _ = bm.set_info(&image_info, width as usize * 4);
    bm.alloc_pixels();
    let src = src.to_rgba8();
    let src_bytes = src.as_bytes();
    unsafe {
        memcpy(bm.pixels(), src_bytes.as_ptr() as *const c_void, src_bytes.len());
    }
    bm.as_image()
}