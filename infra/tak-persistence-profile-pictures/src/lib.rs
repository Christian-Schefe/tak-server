use std::{io::Cursor, path::PathBuf};

use image::DynamicImage;
use tak_server_app::domain::{
    AccountId, RepoError, RepoRetrieveError,
    profile::{ProfilePicture, ProfilePictureFileType, ProfilePictureRepository},
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

    fn get_file_path(&self, account_id: &AccountId) -> (PathBuf, String) {
        let file_name = compute_file_name_hash(account_id);
        let shard_prefix = &file_name[0..2];
        let shard_path = self.file_path.join(shard_prefix);
        (shard_path, format!("{}.webp", file_name))
    }
}

fn compute_file_name_hash(account_id: &AccountId) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(account_id.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[async_trait::async_trait]
impl ProfilePictureRepository for ProfilePictureRepositoryImpl {
    async fn set_profile_picture(
        &self,
        account_id: &AccountId,
        image: DynamicImage,
    ) -> Result<(), RepoError> {
        let (shard_path, file_name) = self.get_file_path(account_id);
        let file_path = shard_path.join(file_name);
        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::WebP)
            .map_err(|e| RepoError::StorageError(format!("Failed to encode image: {}", e)))?;
        tokio::fs::create_dir_all(shard_path).await.map_err(|e| {
            RepoError::StorageError(format!("Failed to create shard directory: {}", e))
        })?;
        tokio::fs::write(file_path, bytes)
            .await
            .map_err(|e| RepoError::StorageError(format!("Failed to write image file: {}", e)))?;
        Ok(())
    }

    async fn get_profile_picture(
        &self,
        account_id: &AccountId,
    ) -> Result<ProfilePicture, RepoRetrieveError> {
        let (shard_path, file_name) = self.get_file_path(account_id);
        let file_path = shard_path.join(file_name);
        let file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|_| RepoRetrieveError::NotFound)?;
        let stream = tokio_util::io::ReaderStream::new(file);
        Ok(ProfilePicture::new(
            Box::new(stream),
            ProfilePictureFileType::WebP,
        ))
    }

    async fn get_default_profile_picture(&self) -> Result<ProfilePicture, RepoRetrieveError> {
        let file_path = self.file_path.join("default_pfp.webp");
        let file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|_| RepoRetrieveError::NotFound)?;
        let stream = tokio_util::io::ReaderStream::new(file);
        Ok(ProfilePicture::new(
            Box::new(stream),
            ProfilePictureFileType::WebP,
        ))
    }
}
