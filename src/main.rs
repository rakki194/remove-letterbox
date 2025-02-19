use anyhow::{Context, Result};
use clap::Parser;
use log::{info, warn};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Command line tool to remove letterboxing from images
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input directory or file path
    #[arg(short, long)]
    input: PathBuf,

    /// Process files recursively if input is a directory
    #[arg(short, long)]
    recursive: bool,

    /// Threshold for letterbox detection (0-255). Higher values will be more aggressive in detecting letterboxes.
    /// Default is 10, which means pixels with RGB values all below 10 are considered part of the letterbox.
    #[arg(short, long, default_value = "10")]
    threshold: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Check if input path exists
    if !args.input.exists() {
        anyhow::bail!("Input path does not exist: {}", args.input.display());
    }

    // Process single file or directory
    if args.input.is_file() {
        process_file(&args.input, args.threshold).await?;
    } else if args.input.is_dir() {
        process_directory(&args.input, args.recursive, args.threshold).await?;
    }

    Ok(())
}

/// Create a processor function that owns the threshold value
fn create_processor<'a>(
    threshold: u8,
) -> impl for<'r> FnOnce(&'r Path) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> + Send + 'a
{
    move |path: &Path| {
        let path = path.to_owned();
        Box::pin(async move { imx::remove_letterbox_with_threshold(&path, threshold).await })
    }
}

/// Process a single image file to remove letterboxing
async fn process_file(path: &Path, threshold: u8) -> Result<()> {
    // Handle JXL files
    if imx::is_jxl_file(path) {
        info!("Processing JXL file: {}", path.display());
        imx::process_jxl_file(path, Some(create_processor(threshold))).await?;
        return Ok(());
    }

    // Handle other image formats
    if !imx::is_image_file(path) {
        warn!("Skipping non-image file: {}", path.display());
        return Ok(());
    }

    info!("Processing image file: {}", path.display());
    imx::remove_letterbox_with_threshold(path, threshold)
        .await
        .with_context(|| format!("Failed to process image file: {}", path.display()))?;

    Ok(())
}

/// Process a directory of image files
async fn process_directory(dir: &Path, recursive: bool, threshold: u8) -> Result<()> {
    async fn process_directory_inner(dir: PathBuf, recursive: bool, threshold: u8) -> Result<()> {
        info!("Processing directory: {}", dir.display());

        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("Failed to read directory entry in: {}", dir.display()))?
        {
            let path = entry.path();
            if path.is_file() {
                process_file(&path, threshold).await?;
            } else if path.is_dir() && recursive {
                let fut = Box::pin(process_directory_inner(path, recursive, threshold));
                fut.await?;
            }
        }

        Ok(())
    }

    process_directory_inner(dir.to_owned(), recursive, threshold).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, ImageBuffer, Rgba};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_image(path: &Path, width: u32, height: u32, with_letterbox: bool) -> Result<()> {
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

        // Fill the image with white pixels
        for y in 0..height {
            for x in 0..width {
                let pixel = if with_letterbox && (y < height / 4 || y > height * 3 / 4) {
                    Rgba([0, 0, 0, 255]) // Black letterbox
                } else {
                    Rgba([255, 255, 255, 255]) // White content
                };
                img.put_pixel(x, y, pixel);
            }
        }

        img.save(path)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_invalid_path() -> Result<()> {
        let result = process_file(Path::new("nonexistent.jpg"), 10).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_non_image() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let non_image = temp_dir.path().join("test.txt");
        fs::write(&non_image, "not an image")?;

        let result = process_file(&non_image, 10).await;
        assert!(result.is_ok()); // Should skip non-image files
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_with_letterbox() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let image_path = temp_dir.path().join("test.png");
        create_test_image(&image_path, 100, 100, true)?;

        process_file(&image_path, 10).await?;

        // Verify the image was processed
        let processed_img = image::open(&image_path)?;
        let (width, height) = processed_img.dimensions();
        assert_eq!(width, 100);
        assert!(height < 100); // Should be cropped
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_without_letterbox() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let image_path = temp_dir.path().join("test.png");
        create_test_image(&image_path, 100, 100, false)?;

        process_file(&image_path, 10).await?;

        // Verify the image was not modified
        let processed_img = image::open(&image_path)?;
        let (width, height) = processed_img.dimensions();
        assert_eq!(width, 100);
        assert_eq!(height, 100); // Should not be cropped
        Ok(())
    }

    #[tokio::test]
    async fn test_process_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create test images in root directory
        let img1 = temp_dir.path().join("test1.png");
        let img2 = temp_dir.path().join("test2.png");
        create_test_image(&img1, 100, 100, true)?;
        create_test_image(&img2, 100, 100, false)?;

        // Create subdirectory with more images
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir)?;
        let img3 = sub_dir.join("test3.png");
        create_test_image(&img3, 100, 100, true)?;

        // Test non-recursive
        process_directory(temp_dir.path(), false, 10).await?;
        let processed_img1 = image::open(&img1)?;
        assert!(processed_img1.dimensions().1 < 100); // Should be cropped
        let processed_img2 = image::open(&img2)?;
        assert_eq!(processed_img2.dimensions().1, 100); // Should not be cropped
        let unprocessed_img3 = image::open(&img3)?;
        assert_eq!(unprocessed_img3.dimensions().1, 100); // Should not be processed

        // Test recursive
        process_directory(temp_dir.path(), true, 10).await?;
        let processed_img3 = image::open(&img3)?;
        assert!(processed_img3.dimensions().1 < 100); // Should be cropped
        Ok(())
    }
}
