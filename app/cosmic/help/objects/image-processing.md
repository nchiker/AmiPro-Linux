---
id: image-processing
title: Image processing
area: objects
keywords: brightness, contrast, edge enhancement, smoothing, invert, negative, revert, tiff, gray scale, adjust image, tgf
see_also: image-halftone, pictures, picture-frame-actions
context: command:ImageProcessing
---
Image processing changes how a bitmap looks without touching the original file. Select a bitmap frame and choose **Tools › Image Processing…**; the rail shows Frame Properties with an **Image** section under Picture. It works on BMP, PNG, JPEG, GIF, TIFF and PCX pictures; metafiles and drawings have no Image section.

## The controls

- **Brightness** and **Contrast**: 0 to 100, with 50 meaning no change.
- **Edge enhancement**: 0 to 100. Raises the crispness of edges; high values overshoot to pure black and white around a step.
- **Smoothing**: 0 to 100. Blends neighbouring pixels to soften jagged edges and noise.
- **Invert**: swaps light and dark, like a photographic negative.
- **Halftone**: a stored print setting; see [Halftone and posterize](image-halftone).

Each setting is a number box with a slider. Type a value and press Enter, or drag the slider; the page redraws while you drag, and the whole drag is one undo step.

**Revert** returns every setting to neutral (50, 50, 0, 0, not inverted). It is available only when something differs.

## Filter order and look

The filters apply in a fixed order: brightness and contrast, smoothing, edge enhancement, invert. The smoothing and edge filters are this program's own, chosen to look like Ami Pro's; results may differ slightly. See [Known limitations](known-limitations).

## What is saved

The settings are kept with the picture in the program's own files, and the original pixels are never changed. When saving as an Ami Pro `.SAM`, an 8-bit gray-scale TIFF keeps its settings in Ami Pro's own form, so Ami Pro applies them. For other bitmaps the settings are baked into the pixels written to the `.SAM`; the panel states which applies.
