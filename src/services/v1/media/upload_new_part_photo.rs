use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{images, new_part_form_image_links, new_part_forms},
};

pub struct UploadNewPartPhotoEffect {
    pub image: images::ActiveModel,
    pub image_link: new_part_form_image_links::ActiveModel,
}

pub fn decide_upload_new_part_photo(
    form: &new_part_forms::Model,
    object_name: String,
) -> Result<UploadNewPartPhotoEffect, AppError> {
    let image_id = Uuid::new_v4();
    let now = Utc::now();

    let image = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(object_name),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    let image_link = new_part_form_image_links::ActiveModel {
        image_id: Set(image_id),
        new_part_form_id: Set(form.id),
    };

    Ok(UploadNewPartPhotoEffect { image, image_link })
}
