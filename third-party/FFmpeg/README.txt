Audio-only FFmpeg, LGPL-2.1-or-later.
Source: FFmpeg commit 46d899d7a02bc63237472cc4897258d5ddf274ac, included as FFmpeg-source.zip.
Toolchain: w64devkit x64 2.10.1 (GCC 16.2.0), https://github.com/skeeto/w64devkit/releases/tag/v2.10.1
Corresponding toolchain sources: https://github.com/skeeto/w64devkit/releases/download/v2.10.1/source.tar
Extract FFmpeg-source.zip into this folder and run sh build-audio.sh inside a w64devkit x64 shell.
No FFmpeg source files were patched; configure flags select native audio decoders, file demuxers,
sample conversion and PCM WAV output. Video codecs, hardware acceleration, network and external
codec libraries are excluded. Replacement FFmpeg tools remain supported in Tidsro.exe's codecs folder.
