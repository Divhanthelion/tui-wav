# WORKING MEMORY

Cross-module knowledge base. Each module leaves notes for modules that depend on it.

## How to Read This File
When implementing a module, find the sections for your dependencies and pay attention to:
- Method signatures (especially return types: Option vs Result, &T vs T)
- Trait implementations you can rely on (FromStr, Clone, etc.)
- Gotchas and non-obvious patterns

## How Notes Are Structured
Each module section contains:
- **Key Types**: The main structs/enums and their purpose
- **Critical Signatures**: Method signatures that are easy to get wrong
- **Trait Impls**: What traits are implemented (use these!)
- **Gotchas**: Things that will break your code if you assume wrong

---

## audio_decoder

**Notes for dependents:**
- AudioDecoder::new() supports WAV files only; MP3/FLAC/OGG return UnsupportedFormat errors
- DecoderError implements std::error::Error and From<std::io::Error>
- sample_rate() and channels() return the metadata parsed from the file header
- read_samples() currently returns an error indicating streaming isn't implemented; use decode_all() instead
- decode_all() is a static method that fully decodes WAV files into Vec<f32>
- Only 8-bit and 16-bit PCM WAV files are supported (audio_format == 1)
- All sample values are normalized to [-1.0, 1.0] range
- Multi-channel audio is downmixed by averaging channels

