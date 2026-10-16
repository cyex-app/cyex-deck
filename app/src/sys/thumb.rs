//! V1051-05 A small PNG of an image, made by Windows' own image codecs (WIC), so no
//! decoder crate is needed. Decodes PNG, JPG and GIF (first frame), and WebP where Windows
//! has its codec.

use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapEncoderNoCache, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, STATSTG,
    STATFLAG_NONAME, STREAM_SEEK_SET,
};
use windows::Win32::UI::Shell::SHCreateMemStream;

/// `bytes` (an image file) scaled down to fit `max` x `max` as PNG; None when it won't decode.
/// Runs on a thread of its own, so COM never meets the caller's model.
pub fn thumbnail(bytes: Vec<u8>, max: u32) -> Option<Vec<u8>> {
    std::thread::spawn(move || {
        // SAFETY: COM for this thread only, balanced below.
        let joined = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        let out = make(&bytes, max).ok();
        if joined {
            // SAFETY: balances the successful CoInitializeEx above.
            unsafe { CoUninitialize() };
        }
        out
    })
    .join()
    .ok()
    .flatten()
}

fn make(bytes: &[u8], max: u32) -> windows::core::Result<Vec<u8>> {
    // SAFETY: plain WIC calls on objects owned by this function; the COM objects drop before
    // the thread leaves COM only because `thumbnail` calls `make` first.
    unsafe {
        let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let input = SHCreateMemStream(Some(bytes)).ok_or_else(windows::core::Error::from_thread)?;
        let decoder = factory.CreateDecoderFromStream(&input, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let frame = decoder.GetFrame(0)?;
        let (mut w, mut h) = (0, 0);
        frame.GetSize(&mut w, &mut h)?;
        if w == 0 || h == 0 {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(-1)));
        }
        let (tw, th) = fit(w, h, max);
        let scaler = factory.CreateBitmapScaler()?;
        scaler.Initialize(&frame, tw, th, WICBitmapInterpolationModeFant)?;
        let pixels = factory.CreateFormatConverter()?;
        pixels.Initialize(
            &scaler,
            &GUID_WICPixelFormat32bppBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let out = SHCreateMemStream(None).ok_or_else(windows::core::Error::from_thread)?;
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        encoder.Initialize(&out, WICBitmapEncoderNoCache)?;
        let mut target = None;
        encoder.CreateNewFrame(&mut target, std::ptr::null_mut())?;
        let target = target.ok_or_else(windows::core::Error::from_thread)?;
        target.Initialize(None)?;
        target.SetSize(tw, th)?;
        let mut format = GUID_WICPixelFormat32bppBGRA;
        target.SetPixelFormat(&mut format)?;
        target.WriteSource(&pixels, std::ptr::null())?;
        target.Commit()?;
        encoder.Commit()?;
        let mut stat = STATSTG::default();
        out.Stat(&mut stat, STATFLAG_NONAME)?;
        out.Seek(0, STREAM_SEEK_SET, None)?;
        let mut png = vec![0u8; stat.cbSize as usize];
        let mut read = 0u32;
        out.Read(png.as_mut_ptr().cast(), png.len() as u32, Some(&mut read)).ok()?;
        png.truncate(read as usize);
        Ok(png)
    }
}

/// `w` x `h` scaled to fit a `max` square, never up.
fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w <= max && h <= max {
        return (w, h);
    }
    let scale = f64::from(max) / f64::from(w.max(h));
    (
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real 2x2 PNG.
    const TINY: [u8; 76] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x06, 0x00, 0x00, 0x00, 0x72, 0xb6, 0x0d,
        0x24, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
        0x1f, 0x0a, 0x19, 0xc0, 0xcc, 0xff, 0xff, 0x01, 0x51, 0xc2, 0x08, 0xf8, 0xbc, 0x4f, 0x72, 0x0f,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn fits_a_square_and_never_scales_up() {
        assert_eq!(fit(1920, 1080, 96), (96, 54));
        assert_eq!(fit(100, 400, 96), (24, 96));
        assert_eq!(fit(50, 20, 96), (50, 20));
        assert_eq!(fit(10_000, 1, 96), (96, 1));
    }

    #[test]
    fn garbage_is_none() {
        assert!(thumbnail(vec![1, 2, 3], 96).is_none());
    }

    #[test]
    fn a_png_comes_back_as_a_png() {
        let out = thumbnail(TINY.to_vec(), 96).expect("WIC decodes a PNG");
        assert_eq!(&out[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }
}
