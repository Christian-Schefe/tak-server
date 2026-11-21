use std::{io::Cursor, path::PathBuf};

use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{ProfilePicture, ProfilePictureFileType},
};
use tak_server_app::{
    errors::{AppResult, InternalError, OneOf},
    ports::profile::ProfilePictureRepository,
};

pub struct ProfilePictureRepositoryImpl {
    file_path: PathBuf,
}

impl ProfilePictureRepositoryImpl {
    pub async fn new() -> Self {
        let file_path = std::env::var("PROFILE_PICTURE_STORAGE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("profile_pictures"));
        tokio::fs::create_dir_all(&file_path)
            .await
            .expect("Failed to create profile picture storage directory");
        Self { file_path }
    }

    fn get_file_path(&self, user_id: UserId) -> (PathBuf, String) {
        let file_name = compute_file_name_hash(user_id);
        let shard_prefix = &file_name[0..2];
        let shard_path = self.file_path.join(shard_prefix);
        (shard_path, format!("{}.webp", file_name))
    }
}

fn compute_file_name_hash(user_id: UserId) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(user_id.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[async_trait::async_trait]
impl ProfilePictureRepository for ProfilePictureRepositoryImpl {
    async fn set_profile_picture(
        &self,
        user_id: UserId,
        image: DynamicImage,
    ) -> AppResult<(), (InternalError,)> {
        let (shard_path, file_name) = self.get_file_path(user_id);
        let file_path = shard_path.join(file_name);
        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::WebP)
            .map_err(|e| {
                tracing::error!("Failed to encode image: {}", e);
                OneOf::new(InternalError)
            })?;
        tokio::fs::create_dir_all(shard_path).await.map_err(|e| {
            tracing::error!("Failed to create shard directory: {}", e);
            OneOf::new(InternalError)
        })?;
        tokio::fs::write(file_path, bytes).await.map_err(|e| {
            tracing::error!("Failed to write image file: {}", e);
            OneOf::new(InternalError)
        })?;
        Ok(())
    }

    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<ProfilePicture>, (InternalError,)> {
        let (shard_path, file_name) = self.get_file_path(user_id);
        let file_path = shard_path.join(file_name);
        let file = match tokio::fs::File::open(&file_path).await {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                tracing::error!("Failed to open profile picture file: {}", e);
                return Err(OneOf::new(InternalError));
            }
        };
        let stream = tokio_util::io::ReaderStream::new(file);
        Ok(Some(ProfilePicture::new(
            Box::new(stream),
            ProfilePictureFileType::WebP,
        )))
    }
}
