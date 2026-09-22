//! Persistent legacy GDI fallback for systems without usable WGC.

use crate::domain::{PhysicalSize, Region};
use crate::pixels::frame::{copy_rgba_frame_bottom_up, BgraFrameMut};

pub(super) struct GdiRegionCapture {
    screen: screenshots::Screen,
    region: Region,
    full_monitor: bool,
    pending_full_frame: Option<Vec<u8>>,
}

impl GdiRegionCapture {
    pub(super) fn new(raw_monitor: isize, region: Region) -> Result<Self, String> {
        let screen = screen_for_monitor(raw_monitor)?;
        Ok(Self {
            screen,
            region,
            full_monitor: false,
            pending_full_frame: None,
        })
    }

    pub(super) fn new_full(raw_monitor: isize) -> Result<Self, String> {
        let screen = screen_for_monitor(raw_monitor)?;
        let captured = screen
            .capture()
            .map_err(|error| format!("Failed to capture monitor with GDI fallback: {error}"))?;
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: captured.width(),
            physical_height: captured.height(),
        };
        Ok(Self {
            screen,
            region,
            full_monitor: true,
            pending_full_frame: Some(captured.into_raw()),
        })
    }

    pub(super) fn region(&self) -> Region {
        self.region
    }

    fn full_monitor_pixels(&mut self) -> Result<Vec<u8>, String> {
        if let Some(pixels) = self.pending_full_frame.take() {
            return Ok(pixels);
        }
        let captured = self
            .screen
            .capture()
            .map_err(|error| format!("Failed to capture monitor with GDI fallback: {error}"))?;
        if captured.width() != self.region.physical_width
            || captured.height() != self.region.physical_height
        {
            return Err(format!(
                "Monitor dimensions changed from {}x{} to {}x{}",
                self.region.physical_width,
                self.region.physical_height,
                captured.width(),
                captured.height()
            ));
        }
        Ok(captured.into_raw())
    }

    pub(super) fn capture_into(
        &mut self,
        output_width: u32,
        output_height: u32,
        output: &mut [u8],
    ) -> Result<(), String> {
        if self.full_monitor {
            let pixels = self.full_monitor_pixels()?;
            return copy_rgba_frame_bottom_up(
                &pixels,
                self.region.size(),
                BgraFrameMut::new(output, PhysicalSize::new(output_width, output_height)),
            );
        }

        let captured = self
            .screen
            .capture_area_ignore_area_check(
                self.region.physical_x,
                self.region.physical_y,
                self.region.physical_width,
                self.region.physical_height,
            )
            .map_err(|error| format!("Failed to capture recording area: {error}"))?;
        if captured.width() != self.region.physical_width
            || captured.height() != self.region.physical_height
        {
            return Err(format!(
                "GDI captured {}x{} but the recording region is {}x{}",
                captured.width(),
                captured.height(),
                self.region.physical_width,
                self.region.physical_height
            ));
        }
        copy_rgba_frame_bottom_up(
            captured.as_raw(),
            PhysicalSize::new(captured.width(), captured.height()),
            BgraFrameMut::new(output, PhysicalSize::new(output_width, output_height)),
        )
    }
}

fn screen_for_monitor(raw_monitor: isize) -> Result<screenshots::Screen, String> {
    let screen = screenshots::Screen::all()
        .map_err(|error| format!("Failed to get screens for GDI fallback: {error}"))?
        .into_iter()
        .find(|screen| screen.display_info.raw_handle.0 == raw_monitor)
        .ok_or_else(|| format!("Monitor handle {raw_monitor:#x} was not found"))?;
    Ok(screen)
}
