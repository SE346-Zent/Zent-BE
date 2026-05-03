#[derive(Clone)]
pub struct MediaService;

impl MediaService {
    pub fn new() -> Self {
        Self
    }

    pub async fn upload_media(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn get_media(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn list_media(&self) -> Result<(), ()> { unimplemented!() }
}
