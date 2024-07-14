use leptess::LepTess;

pub mod image;
pub mod word;

pub fn get_tesseract() -> anyhow::Result<LepTess> {
    Ok(LepTess::new(None, "eng")?)
}
