# Compact mascot animation implementation

Approved design: faithful simplified layered SVG from `assets/mascot.png`, gentle motion for six states, native preview first, shared ESP32 renderer.

## Contract

- SVG authoring only; compiled RGB565 + packed alpha4 cropped sprites, no embedded PNG.
- Maximum mascot blob 128 KiB, existing 19,200-byte tile buffer, additional animation state <=8 KiB.
- Integer fixed-point transforms and eased 350 ms transitions (600 ms entering sleep); interrupt from current pose.
- Six states retain existing USB semantics. Native preview and firmware consume identical poses and pixels.
- Original PNG remains untouched. No flashing, publishing, or commits.

## Work and verification

- [x] Shared core: versioned alpha4 assets, bounded animation controller, tiled compositor. Test easing, interruption, transparency, clipping, deterministic output, memory limits.
- [x] Artwork/compiler: editable SVG body/eyes/mouth/cheeks/shine, cropped shared sprites and deterministic compilation. Test six scenes, deduplication, alpha and 128 KiB budget.
- [x] Consumers: firmware controller integration; native preview preserves transition history through pause/step/seek and changes target without stream restart. Test integration and timeline behavior.
- [x] Evidence: rendered contact sheets and new golden hashes, host/firmware parity, focused frontend tests, workspace checks and target firmware build.
- [x] Launch native app for inspection; document measured asset size, memory, and any unverified physical timing.
