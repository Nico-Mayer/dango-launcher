//! Icon rendering, shared by the application index and the list of running
//! applications so neither extension depends on the other.

use objc2_app_kit::NSWorkspace;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
    CGInterpolationQuality,
};
use objc2_foundation::{NSRect, NSString};

use crate::extensions::applications::IconRgba;

pub fn icon_for(path: &str, size: u32) -> Option<IconRgba> {
    let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
    let mut proposed = NSRect::new(CGPoint::ZERO, CGSize::new(size as f64, size as f64));
    let cg_image =
        unsafe { image.CGImageForProposedRect_context_hints(&mut proposed, None, None) }?;
    render(&cg_image, size)
}

/// Draws the icon into a square RGBA buffer at the requested size. Core
/// Graphics only offers premultiplied alpha for 8-bit RGBA, and PNG wants it
/// straight, so the colour channels are divided back out.
fn render(image: &CGImage, size: u32) -> Option<IconRgba> {
    let space = CGColorSpace::new_device_rgb()?;
    let bytes_per_row = size as usize * 4;
    let mut pixels = vec![0u8; bytes_per_row * size as usize];
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            size as usize,
            size as usize,
            8,
            bytes_per_row,
            Some(&space),
            CGImageAlphaInfo::PremultipliedLast.0,
        )
    }?;
    CGContext::set_interpolation_quality(Some(&context), CGInterpolationQuality::High);
    CGContext::draw_image(
        Some(&context),
        CGRect::new(CGPoint::ZERO, CGSize::new(size as f64, size as f64)),
        Some(image),
    );
    drop(context);

    for px in pixels.as_chunks_mut::<4>().0 {
        let alpha = px[3] as u32;
        if alpha > 0 && alpha < 255 {
            for channel in &mut px[..3] {
                *channel = ((*channel as u32 * 255) / alpha).min(255) as u8;
            }
        }
    }
    Some(IconRgba {
        width: size,
        height: size,
        rgba: pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn an_icon_renders_at_the_requested_size() {
        let icon = icon_for("/System/Applications/Utilities/Terminal.app", 64).unwrap();
        assert_eq!((icon.width, icon.height), (64, 64));
        assert!(icon.rgba.iter().any(|&byte| byte != 0));
    }
}
