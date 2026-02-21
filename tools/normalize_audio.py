#!/usr/bin/env python3
from __future__ import annotations

import argparse
import math
import re
import wave
from pathlib import Path


PARTICLES_RS = Path("src/app/core/particles.rs")
AUDIO_CONST_RE = re.compile(
    r'pub\(super\)\s+const\s+BIRD_AUDIO_PATH:\s*&str\s*=\s*"([^"]+\.wav)";'
)


def parse_audio_path(repo_root: Path) -> Path:
    particles_rs = repo_root / PARTICLES_RS
    content = particles_rs.read_text(encoding="utf-8")
    match = AUDIO_CONST_RE.search(content)
    if not match:
        raise RuntimeError(f"Could not find BIRD_AUDIO_PATH in {particles_rs}")
    return repo_root / match.group(1)


def decode_pcm(raw: bytes, sample_width: int) -> list[int]:
    if sample_width not in (1, 2, 3, 4):
        raise ValueError(f"Unsupported sample width: {sample_width} bytes")

    total_samples = len(raw) // sample_width
    samples: list[int] = [0] * total_samples

    if sample_width == 1:
        for i, value in enumerate(raw):
            samples[i] = value - 128
        return samples

    for i in range(total_samples):
        start = i * sample_width
        chunk = raw[start : start + sample_width]
        samples[i] = int.from_bytes(chunk, byteorder="little", signed=True)
    return samples


def encode_pcm(samples: list[int], sample_width: int) -> bytes:
    if sample_width not in (1, 2, 3, 4):
        raise ValueError(f"Unsupported sample width: {sample_width} bytes")

    if sample_width == 1:
        return bytes(value + 128 for value in samples)

    out = bytearray(len(samples) * sample_width)
    for i, sample in enumerate(samples):
        start = i * sample_width
        out[start : start + sample_width] = int(sample).to_bytes(
            sample_width,
            byteorder="little",
            signed=True,
        )
    return bytes(out)


def normalize_pcm(
    samples: list[int],
    channels: int,
    sample_width: int,
    target_dbfs: float,
    remove_dc_offset: bool,
    independent_channels: bool,
) -> tuple[list[int], float]:
    if channels <= 0:
        raise ValueError("channels must be greater than 0")

    total = len(samples)
    if total == 0:
        return samples, 1.0

    frames = total // channels
    if frames == 0:
        return samples, 1.0

    means = [0.0] * channels
    if remove_dc_offset:
        for idx, sample in enumerate(samples):
            means[idx % channels] += sample
        means = [value / frames for value in means]

    adjusted = [0.0] * total
    for idx, sample in enumerate(samples):
        adjusted[idx] = sample - means[idx % channels]

    bits = sample_width * 8
    min_val = -(1 << (bits - 1))
    max_val = (1 << (bits - 1)) - 1
    full_scale = float(max(abs(min_val), abs(max_val)))
    target_linear = 10.0 ** (target_dbfs / 20.0)
    target_peak = target_linear * full_scale

    if independent_channels:
        peaks = [0.0] * channels
        for idx, value in enumerate(adjusted):
            ch = idx % channels
            abs_val = abs(value)
            if abs_val > peaks[ch]:
                peaks[ch] = abs_val

        gains = [target_peak / peak if peak > 0.0 else 1.0 for peak in peaks]
        applied_gain = max(gains)
    else:
        peak = max(abs(value) for value in adjusted)
        gain = target_peak / peak if peak > 0.0 else 1.0
        gains = [gain] * channels
        applied_gain = gain

    normalized: list[int] = [0] * total
    for idx, value in enumerate(adjusted):
        scaled = int(round(value * gains[idx % channels]))
        if scaled < min_val:
            scaled = min_val
        elif scaled > max_val:
            scaled = max_val
        normalized[idx] = scaled

    return normalized, applied_gain


def normalize_wav(
    wav_path: Path,
    target_dbfs: float,
    remove_dc_offset: bool,
    independent_channels: bool,
) -> float:
    with wave.open(str(wav_path), "rb") as reader:
        params = reader.getparams()
        if params.comptype != "NONE":
            raise RuntimeError(f"Unsupported WAV compression type: {params.comptype}")

        raw = reader.readframes(params.nframes)

    decoded = decode_pcm(raw, params.sampwidth)
    normalized, applied_gain = normalize_pcm(
        samples=decoded,
        channels=params.nchannels,
        sample_width=params.sampwidth,
        target_dbfs=target_dbfs,
        remove_dc_offset=remove_dc_offset,
        independent_channels=independent_channels,
    )

    encoded = encode_pcm(normalized, params.sampwidth)
    temp_path = wav_path.with_suffix(wav_path.suffix + ".tmp")

    with wave.open(str(temp_path), "wb") as writer:
        writer.setparams(params)
        writer.writeframes(encoded)

    temp_path.replace(wav_path)
    return applied_gain


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Normalize the bird WAV referenced by particles.rs using Audacity-like defaults."
    )
    parser.add_argument(
        "--target-dbfs",
        type=float,
        default=-1.0,
        help="Target peak level in dBFS (Audacity default: -1.0).",
    )
    parser.add_argument(
        "--keep-dc-offset",
        action="store_true",
        help="Do not remove DC offset before peak normalization.",
    )
    parser.add_argument(
        "--independent-channels",
        action="store_true",
        help="Normalize each channel independently.",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    wav_path = parse_audio_path(repo_root)

    if not wav_path.exists():
        raise FileNotFoundError(f"Referenced WAV file does not exist: {wav_path}")

    gain = normalize_wav(
        wav_path=wav_path,
        target_dbfs=args.target_dbfs,
        remove_dc_offset=not args.keep_dc_offset,
        independent_channels=args.independent_channels,
    )

    gain_db = 20.0 * math.log10(gain) if gain > 0.0 else float("-inf")
    print(f"Normalized: {wav_path}")
    print(f"Applied gain: {gain:.6f} ({gain_db:+.2f} dB)")


if __name__ == "__main__":
    main()
