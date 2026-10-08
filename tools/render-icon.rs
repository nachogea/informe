//! Rasterize the checked-in SVG into macOS icon sizes using existing dependencies.
use resvg::{tiny_skia, usvg};
use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets = root.join("assets/brand");
    let iconset = root.join("target/brand/AppIcon.iconset");
    fs::create_dir_all(&iconset)?;
    let tree = usvg::Tree::from_str(
        &fs::read_to_string(assets.join("app-icon.svg"))?,
        &usvg::Options::default(),
    )?;
    let render = |size: u32, path: &Path| -> Result<(), Box<dyn Error>> {
        let mut pixels = tiny_skia::Pixmap::new(size, size).ok_or("Invalid icon size")?;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(size as f32 / 1024., size as f32 / 1024.),
            &mut pixels.as_mut(),
        );
        pixels.save_png(path)?;
        Ok(())
    };
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let suffix = if scale == 2 { "@2x" } else { "" };
            render(
                size * scale,
                &iconset.join(format!("icon_{size}x{size}{suffix}.png")),
            )?;
        }
    }
    render(1024, &assets.join("app-icon.png"))?;
    println!("Generated app-icon.png and target/brand/AppIcon.iconset");
    Ok(())
}
