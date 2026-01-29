use std::path::Path;

use image::GenericImageView;
use image_compare::Algorithm;
use tracing::{debug, info};

use super::errors::DiffError;
use super::types::{DiffRequest, DiffResult};

/// Compare two images and calculate their similarity using SSIM (Structural Similarity Index)
///
/// # Errors
///
/// Returns [`DiffError::ImageLoadFailed`] if either image cannot be loaded (file not found,
/// invalid format, or permission denied).
///
/// Returns [`DiffError::DimensionMismatch`] if the images have different dimensions.
///
/// Returns [`DiffError::ComparisonFailed`] if the SSIM calculation fails.
pub fn compare_images(request: &DiffRequest) -> Result<DiffResult, DiffError> {
    info!(
        event = "core.diff.compare_started",
        image1 = %request.image1_path.display(),
        image2 = %request.image2_path.display(),
        threshold = request.threshold
    );

    // Load images
    let img1 = image::open(&request.image1_path).map_err(|e| DiffError::ImageLoadFailed {
        path: request.image1_path.display().to_string(),
        message: e.to_string(),
    })?;

    let img2 = image::open(&request.image2_path).map_err(|e| DiffError::ImageLoadFailed {
        path: request.image2_path.display().to_string(),
        message: e.to_string(),
    })?;

    let (width1, height1) = img1.dimensions();
    let (width2, height2) = img2.dimensions();

    // Check dimensions match
    if width1 != width2 || height1 != height2 {
        return Err(DiffError::DimensionMismatch {
            width1,
            height1,
            width2,
            height2,
        });
    }

    // Convert to grayscale for SSIM comparison
    let gray1 = img1.to_luma8();
    let gray2 = img2.to_luma8();

    // Calculate SSIM (Structural Similarity Index)
    let result = image_compare::gray_similarity_structure(&Algorithm::MSSIMSimple, &gray1, &gray2)
        .map_err(|e| DiffError::ComparisonFailed(e.to_string()))?;

    let similarity = result.score;

    // Generate visual diff image if output path requested
    let diff_output_path = if let Some(ref output_path) = request.diff_output_path {
        save_diff_image(&img1, &img2, output_path)?;
        Some(output_path.display().to_string())
    } else {
        None
    };

    let diff_result = DiffResult::new(
        similarity,
        width1,
        height1,
        width2,
        height2,
        request.threshold,
        diff_output_path,
    );

    info!(
        event = "core.diff.compare_completed",
        similarity = similarity,
        is_similar = diff_result.is_similar()
    );

    Ok(diff_result)
}

/// Compute per-pixel absolute differences between two images and save as PNG
fn save_diff_image(
    img1: &image::DynamicImage,
    img2: &image::DynamicImage,
    output_path: &Path,
) -> Result<(), DiffError> {
    info!(
        event = "core.diff.save_started",
        path = %output_path.display()
    );

    let img1_rgba = img1.to_rgba8();
    let img2_rgba = img2.to_rgba8();
    let (width, height) = img1.dimensions();

    let mut diff_img = image::RgbImage::new(width, height);
    for (x, y, p1) in img1_rgba.enumerate_pixels() {
        let p2 = img2_rgba.get_pixel(x, y);
        diff_img.put_pixel(
            x,
            y,
            image::Rgb([
                p1[0].abs_diff(p2[0]),
                p1[1].abs_diff(p2[1]),
                p1[2].abs_diff(p2[2]),
            ]),
        );
    }

    // Create parent directory if needed
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        debug!(
            event = "core.diff.creating_parent_directory",
            path = %parent.display()
        );
        std::fs::create_dir_all(parent)?;
    }

    image::DynamicImage::ImageRgb8(diff_img)
        .save(output_path)
        .map_err(|e| DiffError::DiffGenerationFailed(e.to_string()))?;

    info!(
        event = "core.diff.save_completed",
        path = %output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::PeekError;
    use std::path::PathBuf;

    #[test]
    fn test_compare_nonexistent_image() {
        let request = DiffRequest::new(
            PathBuf::from("/nonexistent/image1.png"),
            PathBuf::from("/nonexistent/image2.png"),
        );
        let result = compare_images(&request);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_code(), "DIFF_IMAGE_LOAD_FAILED");
        }
    }
}
