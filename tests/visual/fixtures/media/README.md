# Synthetic playback fixtures

These original test signals contain no third-party footage. Generated with FFmpeg's `testsrc2` (160 × 90, 15 fps) and 440 Hz `sine` filters; duration 24 seconds. The MP4 contains H.264 picture and quiet AAC audio. The M4A intentionally contains only AAC audio, to reproduce an advancing audio clock with no decoded video frame.

They exercise actual browser decoding and media cleanup; metadata and playback clocks alone do not constitute a successful picture test.
